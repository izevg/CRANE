mod endpoint;
mod inbox;
mod waiters;

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crane_ports::proxy::{Cached, CachedState};
use fred::clients::{Client, SubscriberClient, WithOptions};
use fred::interfaces::ClientLike;
use fred::types::config::{ConnectionConfig, Options, ReconnectPolicy};
use moka::sync::Cache;
use tokio::task::JoinHandle;

pub use endpoint::{Endpoint, EndpointError};
pub(crate) use waiters::Waiters;

use crate::proxy::Releases;

/// Условные накладные расходы записи сверх тела: ключ, счётчики, метки времени.
const ENTRY_OVERHEAD_BYTES: u32 = 64;

/// Канал инвалидации: payload — имя ключа записи кэша.
pub(crate) const INVALIDATION_CHANNEL: &str = "inv";

/// Потолок времени одной команды. Операции кэша - микросекунды в одной сети, поэтому секунда
/// с запасом покрывает любой здоровый ответ, а зависший сокет не держит запрос движка дольше:
/// `get` и `claim` отвечают ошибкой, и движок идёт в upstream. Put очень большого документа
/// (до 64 мегабайт) может не уложиться и вернуть ошибку: движок её игнорирует (кэш не источник
/// истины), а запись остаётся в L1.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(1);

/// Таймаут установки TCP/TLS-соединения: недоступный хост не должен занимать попытку надолго.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);

/// Реконнект: без лимита попыток, экспоненциально от 100 мс до 5 с (мс, мс, база).
const RECONNECT_MIN_MS: u32 = 100;
const RECONNECT_MAX_MS: u32 = 5_000;
const RECONNECT_BASE: u32 = 2;

/// Ошибка команды Valkey. Несёт только вид ошибки клиента: её текст может цитировать адрес.
#[derive(Debug, Clone, thiserror::Error)]
#[error("команда Valkey не выполнена: {0}")]
pub(crate) struct CommandError(&'static str);

impl From<fred::error::Error> for CommandError {
    fn from(error: fred::error::Error) -> Self {
        Self(error.kind().to_str())
    }
}

/// Задача, которая прекращается вместе с владельцем.
pub(crate) struct AbortOnDrop<T>(pub(crate) JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

fn weight(entry: &Cached) -> u32 {
    let payload = match &entry.state {
        CachedState::Document { body, etag, .. } => body
            .len()
            .saturating_add(etag.as_ref().map_or(0, |etag| etag.as_str().len())),
        CachedState::Blob(_) | CachedState::Missing { .. } => 0,
    };
    u32::try_from(payload).map_or(u32::MAX, |payload| {
        ENTRY_OVERHEAD_BYTES.saturating_add(payload)
    })
}

/// Состояние процесса, которое трогает диспетчер `inv`: L1, поколение и ждущие.
pub(crate) struct Local {
    pub(crate) l1: Cache<String, Cached>,
    pub(crate) waiters: Waiters,
    /// Растёт на каждое сообщение `inv` и на потерю связи с подпиской. `get` по нему замечает,
    /// что запись, прочитанная из L2, могла устареть, пока она шла в L1.
    generation: AtomicU64,
}

impl Local {
    fn new(l1_bytes: u64) -> Self {
        Self {
            l1: Cache::builder()
                .max_capacity(l1_bytes)
                .weigher(|_name: &String, entry: &Cached| weight(entry))
                .build(),
            waiters: Waiters::default(),
            generation: AtomicU64::new(0),
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }

    /// Кладёт в L1 запись, прочитанную из L2, когда поколение было `seen`. Если с тех пор
    /// пришла инвалидация, запись могла устареть, пока шла в L1: её нельзя оставлять.
    pub(crate) fn admit(&self, name: &str, entry: Cached, seen: u64) {
        self.l1.insert(name.to_owned(), entry);
        if self.generation() != seen {
            self.l1.invalidate(name);
        }
    }

    /// Порядок важен: сначала поколение и L1, потом пробуждение. Проснувшийся ждущий
    /// перечитывает кэш и не должен увидеть старую запись в L1.
    pub(crate) fn invalidate(&self, name: &str) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.l1.invalidate(name);
        self.waiters.wake(name);
    }

    /// Сообщения могли потеряться (переподключение, переполнение буфера): L1 нельзя доверять.
    pub(crate) fn forget_all(&self) {
        self.generation.fetch_add(1, Ordering::AcqRel);
        self.l1.invalidate_all();
        self.waiters.wake_all();
    }
}

/// Разделяемая часть экземпляра.
pub(crate) struct Node {
    pub(crate) local: Arc<Local>,
    /// Один клиент (одно соединение): порядок команд процесса сохраняется, на нём держатся
    /// гарантии `claim`/`release`. Быстрый отказ без соединения и потолок времени команды.
    pub(crate) commands: WithOptions<Client>,
    pub(crate) releases: Arc<Releases>,
    _tasks: [AbortOnDrop<()>; 2],
}

/// Кэш записей proxy и межрепликовые lease на Valkey.
#[derive(Clone)]
pub struct Valkey(pub(crate) Arc<Node>);

impl fmt::Debug for Valkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Valkey").finish_non_exhaustive()
    }
}

impl Valkey {
    /// Запускает подключение и диспетчер инвалидации в фоне и сразу возвращается: недоступный
    /// Valkey не роняет старт (`docs/product/16`), а команды до подключения отвечают ошибкой.
    /// `l1_bytes` — бюджет L1 в байтах веса записей.
    ///
    /// Требует tokio-рантайм: фоновые задачи запускаются через `tokio::spawn`.
    #[must_use]
    pub fn start(endpoint: Endpoint, l1_bytes: u64) -> Self {
        let mut config = endpoint.0;
        // Первое подключение подчиняется той же политике реконнекта, а не роняет `init`.
        config.fail_fast = false;
        let connection = ConnectionConfig {
            connection_timeout: CONNECT_TIMEOUT,
            ..ConnectionConfig::default()
        };
        let policy =
            ReconnectPolicy::new_exponential(0, RECONNECT_MIN_MS, RECONNECT_MAX_MS, RECONNECT_BASE);
        let commands = Client::new(
            config.clone(),
            None,
            Some(connection.clone()),
            Some(policy.clone()),
        );
        let subscriber = SubscriberClient::new(config, None, Some(connection), Some(policy));

        let local = Arc::new(Local::new(l1_bytes));
        let tasks = [
            AbortOnDrop(tokio::spawn(hold_connection(commands.clone()))),
            AbortOnDrop(tokio::spawn(inbox::run(Arc::clone(&local), subscriber))),
        ];
        let options = Options {
            timeout: Some(COMMAND_TIMEOUT),
            fail_fast: true,
            ..Options::default()
        };
        Self(Arc::new(Node {
            local,
            commands: commands.with_options(&options),
            releases: Arc::default(),
            _tasks: tasks,
        }))
    }
}

/// Держит соединение командного клиента, пока жив узел: когда задача соединения fred
/// завершилась (политика сдалась, отказ аутентификации), подключает клиента заново.
async fn hold_connection(client: Client) {
    loop {
        let mut connection = inbox::connect(&client).await;
        inbox::connection_lost((&mut connection.0).await).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    const NAME: &str = "px:name";

    fn entry() -> Cached {
        Cached {
            fetched_at: SystemTime::UNIX_EPOCH,
            state: CachedState::Missing { misses: 1 },
        }
    }

    #[test]
    fn admit_keeps_entry_when_nothing_was_invalidated_meanwhile() {
        // Arrange
        let local = Local::new(1 << 20);
        let seen = local.generation();

        // Act
        local.admit(NAME, entry(), seen);

        // Assert
        assert_eq!(local.l1.get(NAME), Some(entry()));
    }

    #[test]
    fn admit_drops_entry_read_before_an_invalidation() {
        // Arrange
        let local = Local::new(1 << 20);
        let seen = local.generation();
        local.invalidate("px:other");

        // Act
        local.admit(NAME, entry(), seen);

        // Assert
        assert_eq!(
            local.l1.get(NAME),
            None,
            "любая инвалидация после чтения L2 делает прочитанное подозрительным"
        );
    }

    #[test]
    fn invalidate_removes_only_the_named_entry() {
        // Arrange
        let local = Local::new(1 << 20);
        local.l1.insert(NAME.to_owned(), entry());
        local.l1.insert("px:other".to_owned(), entry());

        // Act
        local.invalidate(NAME);

        // Assert
        assert_eq!(
            (local.l1.get(NAME), local.l1.get("px:other")),
            (None, Some(entry())),
            "инвалидация выбрасывает только названный ключ"
        );
    }

    #[test]
    fn forget_all_empties_l1() {
        // Arrange
        let local = Local::new(1 << 20);
        local.l1.insert(NAME.to_owned(), entry());
        local.l1.insert("px:other".to_owned(), entry());

        // Act
        local.forget_all();

        // Assert
        assert_eq!((local.l1.get(NAME), local.l1.get("px:other")), (None, None));
    }
}
