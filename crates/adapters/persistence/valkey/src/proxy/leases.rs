use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use crane_domain::proxy::ProxyKey;
use crane_ports::proxy::{Claim, FetchLeases, Lease, LeaseError};
use fred::clients::{Client, WithOptions};
use fred::interfaces::{ClientLike, KeysInterface};
use fred::types::SetOptions;
use fred::types::{ClusterHash, CustomCommand};
use futures_util::FutureExt;
use futures_util::future::{BoxFuture, Shared};
use uuid::Uuid;

use super::{entry_key, expiration, lease_key};
use crate::node::{CommandError, INVALIDATION_CHANNEL, Node, Valkey};

/// Снимает lease, только если в нём ещё наш токен (фенсинг), и будит ждущих на всех репликах.
/// Чужой или истёкший токен - no-op без `PUBLISH`. Один ключ: подходит и для Cluster.
const RELEASE_SCRIPT: &str = "\
if redis.call('GET', KEYS[1]) == ARGV[1] then \
redis.call('DEL', KEYS[1]); \
redis.call('PUBLISH', ARGV[3], ARGV[2]); \
return 1 \
end; \
return 0";

fn backend(error: CommandError) -> LeaseError {
    LeaseError::Backend(Box::new(error))
}

/// Снятие lease, ещё не завершённое в этом процессе; общий результат для всех ждущих.
pub(crate) type Releasing = Shared<BoxFuture<'static, Result<(), CommandError>>>;

/// Незавершённые снятия этого процесса по имени записи: барьер для следующего `claim`.
#[derive(Default)]
pub(crate) struct Releases(Mutex<HashMap<String, Releasing>>);

impl Releases {
    // Замок берётся только в синхронных функциях: guard не живёт через await.
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Releasing>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn pending(&self, name: &str) -> Option<Releasing> {
        self.lock().get(name).cloned()
    }

    fn start(&self, name: String, release: Releasing) {
        self.lock().insert(name, release);
    }

    /// Убирает запись, только если она всё ещё наша: за это время могло начаться новое снятие.
    fn finish(&self, name: &str, release: &Releasing) {
        let mut pending = self.lock();
        if pending
            .get(name)
            .is_some_and(|stored| Shared::ptr_eq(stored, release))
        {
            pending.remove(name);
        }
    }
}

/// Ждёт собственное незавершённое снятие того же ключа. Без снятия не ждёт вовсе, поэтому
/// регистрация и `SET` в `claim` не откладываются на лишний опрос.
async fn await_own_release(node: &Node, name: &str) {
    if let Some(pending) = node.releases.pending(name) {
        // Ошибку снятия получает тот, кто его запросил.
        let _ = pending.await;
    }
}

/// Позиция ключа lease среди аргументов `EVAL` (для хеш-слота в Cluster).
const KEY_OFFSET: usize = 2;

async fn run_release(
    commands: WithOptions<Client>,
    lease_name: String,
    token: u64,
    entry_name: String,
) -> Result<(), CommandError> {
    let eval = CustomCommand::new_static("EVAL", ClusterHash::Offset(KEY_OFFSET), false);
    let args = vec![
        RELEASE_SCRIPT.to_owned(),
        "1".to_owned(),
        lease_name,
        token.to_string(),
        entry_name,
        INVALIDATION_CHANNEL.to_owned(),
    ];
    commands.custom::<i64, _>(eval, args).await?;
    Ok(())
}

impl FetchLeases for Valkey {
    async fn claim(
        &self,
        key: &ProxyKey,
        lease: Duration,
        patience: Duration,
    ) -> Result<Claim, LeaseError> {
        let node = &self.0;
        let name = entry_key(key);
        await_own_release(node, &name).await;
        // Регистрация до SET: `release` лидера, пришедший между SET и ожиданием, не теряется.
        let mut registration = node.local.waiters.register(&name);
        let token = Uuid::new_v4().as_u64_pair().0;
        let acquired: Option<String> = node
            .commands
            .set(
                lease_key(key),
                token.to_string(),
                expiration(lease),
                Some(SetOptions::NX),
                false,
            )
            .await
            .map_err(|error| backend(error.into()))?;
        if acquired.is_some() {
            return Ok(Claim::Lead(Lease::new(key.clone(), token)));
        }
        registration.wait(patience).await;
        Ok(Claim::Settled)
    }

    fn release(&self, lease: Lease) -> impl Future<Output = Result<(), LeaseError>> + Send {
        let node = &self.0;
        let name = entry_key(lease.key());
        let release: Releasing = run_release(
            node.commands.clone(),
            lease_key(lease.key()),
            lease.token(),
            name.clone(),
        )
        .boxed()
        .shared();
        node.releases.start(name.clone(), release.clone());
        // Снятие идёт в фоне уже сейчас: брошенный (неопрошенный) future не отменяет его,
        // а следующий `claim` этого ключа ждёт его завершения.
        let releases = Arc::clone(&node.releases);
        let running = release.clone();
        tokio::spawn(async move {
            let _ = running.clone().await;
            releases.finish(&name, &running);
        });
        async move { release.await.map_err(backend) }
    }
}
