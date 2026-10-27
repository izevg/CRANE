//! Помощники интеграционных тестов с реальным Valkey.
//!
//! `CRANE_TEST_VALKEY_URL` - адрес сервера без номера БД (например `valkey://127.0.0.1:6379`).
//! Без переменной тесты сообщают о пропуске и завершаются успешно, чтобы обычный `cargo test`
//! работал без Docker.
//!
//! `CRANE_TEST_REQUIRE_VALKEY=1` запрещает пропуск: без `CRANE_TEST_VALKEY_URL` тест падает.
//! Так заданы `task rust:test:valkey` и `task gate`: молчаливый пропуск превратил бы
//! мутационный гейт в проверку без единого теста с Valkey.
//!
//! Каждый тест берёт свою БД Valkey (`/<n>`) и очищает её при создании экземпляров. Pub/sub
//! в Valkey общий для всех БД: чужая инвалидация даёт лишь ранний `Settled` или лишнее чтение L2,
//! что порты допускают.

#![allow(
    clippy::expect_used,
    reason = "помощники тестов: сбой окружения или фикстуры должен ронять тест"
)]

use std::time::Duration;

use crane_domain::proxy::{ProxyKey, UpstreamPath};
use crane_domain::registry::RepositoryId;
use crane_ports::proxy::ProxyCache;
use crane_valkey::{Endpoint, Valkey};
use fred::clients::Client;
use fred::interfaces::ClientLike;
use fred::types::config::Config;
use fred::types::{ClusterHash, CustomCommand};
use uuid::Uuid;

const URL_ENV: &str = "CRANE_TEST_VALKEY_URL";
const REQUIRE_ENV: &str = "CRANE_TEST_REQUIRE_VALKEY";

/// Бюджет L1 тестовых экземпляров, байт: хватает с большим запасом.
const L1_BYTES: u64 = 1 << 20;

/// Предел ожидания подписки диспетчера на `inv`.
const SUBSCRIPTION_DEADLINE: Duration = Duration::from_secs(5);
const POLL_STEP: Duration = Duration::from_millis(10);

pub(crate) struct TestServer {
    base: String,
}

/// Выполняет `body` на рантайме с несколькими потоками; без `CRANE_TEST_VALKEY_URL` пропускает
/// тест, а при `CRANE_TEST_REQUIRE_VALKEY=1` падает.
pub(crate) fn run(body: impl AsyncFnOnce(&TestServer)) {
    let Ok(base) = std::env::var(URL_ENV) else {
        assert!(
            std::env::var(REQUIRE_ENV).as_deref() != Ok("1"),
            "{REQUIRE_ENV}=1, но {URL_ENV} не задана: тесты с Valkey нельзя пропускать"
        );
        report_skip();
        return;
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime тестов");
    let server = TestServer {
        base: base.trim_end_matches('/').to_owned(),
    };
    runtime.block_on(body(&server));
}

#[allow(
    clippy::print_stderr,
    reason = "единственный способ сообщить о пропуске: тест при этом завершается успешно"
)]
fn report_skip() {
    eprintln!("ПРОПУСК: {URL_ENV} не задана, тесты с Valkey не выполняются");
}

/// Ждёт, пока командное соединение экземпляра поднимется: до этого команды по замыслу
/// отвечают быстрой ошибкой. Опрос ограничен `SUBSCRIPTION_DEADLINE`.
async fn await_commands(valkey: &Valkey) {
    let probe = ProxyKey {
        repository: RepositoryId::new(Uuid::nil()),
        path: UpstreamPath::parse("readiness/probe").expect("путь пробы валиден"),
    };
    let deadline = tokio::time::Instant::now() + SUBSCRIPTION_DEADLINE;
    while valkey.get(&probe).await.is_err() && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(POLL_STEP).await;
    }
}

impl TestServer {
    fn url(&self, db: u8) -> String {
        format!("{}/{db}", self.base)
    }

    fn start(&self, db: u8) -> Valkey {
        let endpoint = Endpoint::parse(&self.url(db)).expect("CRANE_TEST_VALKEY_URL валиден");
        Valkey::start(endpoint, L1_BYTES)
    }

    /// Чистая БД и один экземпляр с установленной подпиской на `inv`.
    pub(crate) async fn fresh(&self, db: u8) -> Valkey {
        self.flush(db).await;
        let valkey = self.start(db);
        await_commands(&valkey).await;
        self.await_subscribers(db, 1).await;
        valkey
    }

    /// Чистая БД и две «реплики»: два независимых экземпляра на одной БД.
    pub(crate) async fn replicas(&self, db: u8) -> (Valkey, Valkey) {
        self.flush(db).await;
        let replicas = (self.start(db), self.start(db));
        await_commands(&replicas.0).await;
        await_commands(&replicas.1).await;
        self.await_subscribers(db, 2).await;
        replicas
    }

    async fn flush(&self, db: u8) {
        let admin = self.admin(db).await;
        let flush = CustomCommand::new_static("FLUSHDB", ClusterHash::Random, false);
        admin
            .custom::<(), String>(flush, vec![])
            .await
            .expect("FLUSHDB");
        admin.quit().await.expect("закрытие админ-клиента");
    }

    /// Ждёт, пока в БД появятся `expected` подписанных соединений (`CLIENT LIST`: `sub=`).
    /// Подписка ставится диспетчером в фоне, а публичного признака готовности у адаптера нет;
    /// без ожидания первая инвалидация теста могла бы уйти мимо ещё не подписанной реплики.
    /// Опрос ограничен `SUBSCRIPTION_DEADLINE`; по исходу падает следующая проверка теста.
    async fn await_subscribers(&self, db: u8, expected: usize) {
        let admin = self.admin(db).await;
        let list = CustomCommand::new_static("CLIENT", ClusterHash::Random, false);
        let marker = format!("db={db} ");
        let deadline = tokio::time::Instant::now() + SUBSCRIPTION_DEADLINE;
        loop {
            let report: String = admin
                .custom(list.clone(), vec!["LIST"])
                .await
                .expect("CLIENT LIST");
            let subscribed = report
                .lines()
                .filter(|line| format!("{line} ").contains(&marker))
                .filter(|line| {
                    line.split(' ')
                        .any(|field| field.starts_with("sub=") && field != "sub=0")
                })
                .count();
            if subscribed >= expected || tokio::time::Instant::now() >= deadline {
                break;
            }
            tokio::time::sleep(POLL_STEP).await;
        }
        admin.quit().await.expect("закрытие админ-клиента");
    }

    async fn admin(&self, db: u8) -> Client {
        let config = Config::from_url(&self.url(db)).expect("CRANE_TEST_VALKEY_URL валиден");
        let client = Client::new(config, None, None, None);
        client.init().await.expect("подключение администратора");
        client
    }
}
