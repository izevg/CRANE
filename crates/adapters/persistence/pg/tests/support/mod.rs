//! Помощники интеграционных тестов с реальным PostgreSQL.
//!
//! `CRANE_TEST_DATABASE_URL` - URL администратора сервера (подойдёт любая существующая БД,
//! например `postgres://crane:change-me@127.0.0.1:5432/crane`). Без переменной тесты
//! сообщают о пропуске и завершаются успешно, чтобы обычный `cargo test` работал без Docker.
//!
//! `CRANE_TEST_REQUIRE_DB=1` запрещает пропуск: без `CRANE_TEST_DATABASE_URL` тест падает.
//! Так задан `task rust:test:pg` и `task gate`: молчаливый пропуск там превратил бы
//! мутационный гейт в проверку без единого DB-теста.
//!
//! На каждый тестовый процесс создаётся шаблон `crane_tpl_<uuid>` с применёнными миграциями,
//! на каждый сценарий - БД `crane_test_<uuid>` из шаблона (копирование файлов дешевле
//! повторных миграций). В конце все созданные БД удаляются. Если тест упал по panic, БД
//! остаются: чистить по префиксу при старте нельзя, это убило бы БД параллельных процессов.

#![allow(
    clippy::expect_used,
    reason = "помощники тестов: сбой окружения или фикстуры должен ронять тест"
)]

use std::str::FromStr;
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use crane_domain::registry::{
    Asset, Format, FormatMetadata, PackageName, Repository, RepositoryId, RepositoryKind, Sha256,
    Version, VersionId, VersionState,
};
use crane_pg::{PgCatalog, migrate};
use crane_ports::registry::{Catalog, NewVersion};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};
use uuid::Uuid;

const URL_ENV: &str = "CRANE_TEST_DATABASE_URL";
const REQUIRE_ENV: &str = "CRANE_TEST_REQUIRE_DB";

pub(crate) struct TestServer {
    admin: PgPool,
    options: PgConnectOptions,
    template: String,
    created: Mutex<Vec<String>>,
}

/// Выполняет `body` на свежем сервере; без `CRANE_TEST_DATABASE_URL` пропускает тест,
/// а при `CRANE_TEST_REQUIRE_DB=1` падает.
pub(crate) fn run(body: impl AsyncFnOnce(&TestServer)) {
    let Ok(url) = std::env::var(URL_ENV) else {
        assert!(
            std::env::var(REQUIRE_ENV).as_deref() != Ok("1"),
            "{REQUIRE_ENV}=1, но {URL_ENV} не задана: тесты с PostgreSQL нельзя пропускать"
        );
        report_skip();
        return;
    };
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime тестов");
    runtime.block_on(async {
        let server = TestServer::start(&url).await;
        body(&server).await;
        server.shutdown().await;
    });
}

#[allow(
    clippy::print_stderr,
    reason = "единственный способ сообщить о пропуске: тест при этом завершается успешно"
)]
fn report_skip() {
    eprintln!("ПРОПУСК: {URL_ENV} не задана, тесты с PostgreSQL не выполняются");
}

/// Имена БД собираются только из hex uuid: идентификаторы нельзя передать параметром.
fn unique_name(prefix: &str) -> String {
    format!("{prefix}_{}", Uuid::new_v4().simple())
}

impl TestServer {
    async fn start(url: &str) -> Self {
        let options = PgConnectOptions::from_str(url).expect("CRANE_TEST_DATABASE_URL валиден");
        let admin = PgPoolOptions::new()
            .max_connections(4)
            .connect_with(options.clone())
            .await
            .expect("подключение администратора");
        let template = unique_name("crane_tpl");
        create_database(&admin, &template, None).await;
        let pool = connect(&options, &template).await;
        migrate(&pool).await.expect("миграции шаблона");
        // У шаблона не должно остаться сессий, иначе CREATE DATABASE ... TEMPLATE откажет.
        pool.close().await;
        Self {
            admin,
            options,
            template,
            created: Mutex::new(Vec::new()),
        }
    }

    /// Пул к новой БД с применёнными миграциями.
    pub(crate) async fn fresh_pool(&self) -> PgPool {
        self.pool_from(Some(&self.template)).await
    }

    /// Пул к новой БД без единой таблицы.
    pub(crate) async fn empty_pool(&self) -> PgPool {
        self.pool_from(None).await
    }

    pub(crate) async fn fresh_catalog(&self) -> PgCatalog {
        PgCatalog::new(self.fresh_pool().await)
    }

    async fn pool_from(&self, template: Option<&str>) -> PgPool {
        let name = unique_name("crane_test");
        create_database(&self.admin, &name, template).await;
        self.created.lock().expect("список БД").push(name.clone());
        connect(&self.options, &name).await
    }

    async fn shutdown(self) {
        let names = self.created.into_inner().expect("список БД");
        for name in names.iter().chain([&self.template]) {
            sqlx::query(AssertSqlSafe(format!(
                "DROP DATABASE IF EXISTS {name} WITH (FORCE)"
            )))
            .execute(&self.admin)
            .await
            .expect("удаление БД");
        }
        self.admin.close().await;
    }
}

async fn create_database(admin: &PgPool, name: &str, template: Option<&str>) {
    let sql = template.map_or_else(
        || format!("CREATE DATABASE {name}"),
        |t| format!("CREATE DATABASE {name} TEMPLATE {t}"),
    );
    sqlx::query(AssertSqlSafe(sql))
        .execute(admin)
        .await
        .expect("создание БД");
}

async fn connect(options: &PgConnectOptions, database: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(16)
        .connect_with(options.clone().database(database))
        .await
        .expect("подключение к тестовой БД")
}

pub(crate) fn name(raw: &str) -> PackageName {
    PackageName::parse(Format::Cargo, raw).expect("имя пакета валидно")
}

pub(crate) fn version(raw: &str) -> Version {
    Version::parse(Format::Cargo, raw).expect("версия валидна")
}

pub(crate) fn asset(path: &str, fill: u8, size: u64) -> Asset {
    Asset {
        path: path.to_owned(),
        sha256: Sha256::from([fill; 32]),
        size,
    }
}

/// Версия с одним файлом `<package>-<raw>.crate`; `seq` задаёт id и секунду публикации.
pub(crate) fn new_version(seq: u64, repo: RepositoryId, package: &str, raw: &str) -> NewVersion {
    NewVersion {
        id: VersionId::new(Uuid::from_u128(seq.into())),
        repository: repo,
        package: name(package),
        version: version(raw),
        state: VersionState::Quarantined,
        published_at: SystemTime::UNIX_EPOCH + Duration::from_secs(seq),
        assets: vec![asset(&format!("{package}-{raw}.crate"), 7, 3)],
        metadata: FormatMetadata::new(r#"{"deps":[]}"#.to_owned()),
    }
}

pub(crate) async fn hosted_repository(catalog: &impl Catalog) -> RepositoryId {
    catalog
        .ensure_repository(Repository {
            id: RepositoryId::new(Uuid::from_u128(1)),
            format: Format::Cargo,
            kind: RepositoryKind::Hosted,
            name: "main".to_owned(),
        })
        .await
        .expect("репозиторий создаётся")
        .id
}

/// Дожидается всех задач; порядок результатов совпадает с порядком задач.
pub(crate) async fn join_all<T>(tasks: Vec<tokio::task::JoinHandle<T>>) -> Vec<T> {
    let mut results = Vec::with_capacity(tasks.len());
    for task in tasks {
        results.push(task.await.expect("задача завершилась без паники"));
    }
    results
}
