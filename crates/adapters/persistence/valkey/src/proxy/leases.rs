use std::time::Duration;

use crane_domain::proxy::ProxyKey;
use crane_ports::proxy::{Claim, FetchLeases, Lease, LeaseError};
use fred::clients::{Client, WithOptions};
use fred::interfaces::{ClientLike, KeysInterface};
use fred::types::SetOptions;
use fred::types::{ClusterHash, CustomCommand};
use futures_util::FutureExt;
use futures_util::future::Shared;
use uuid::Uuid;

use super::{entry_key, expiration, lease_key};
use crate::node::{CommandError, INVALIDATION_CHANNEL, Node, Releasing, Valkey};

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

/// Ждёт собственное незавершённое снятие того же ключа. Без снятия не ждёт вовсе, поэтому
/// регистрация и `SET` в `claim` не откладываются на лишний опрос.
async fn await_own_release(node: &Node, name: &str) {
    let pending = node
        .releasing
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(name)
        .cloned();
    if let Some(pending) = pending {
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
        node.releasing
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(name.clone(), release.clone());
        // Снятие идёт в фоне уже сейчас: брошенный (неопрошенный) future не отменяет его,
        // а следующий `claim` этого ключа ждёт его завершения.
        let map = std::sync::Arc::clone(&node.releasing);
        let running = release.clone();
        tokio::spawn(async move {
            let _ = running.clone().await;
            let mut map = map
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            // Запись могла уже смениться более новым снятием: чужую не трогаем.
            if map
                .get(&name)
                .is_some_and(|stored| Shared::ptr_eq(stored, &running))
            {
                map.remove(&name);
            }
        });
        async move { release.await.map_err(backend) }
    }
}
