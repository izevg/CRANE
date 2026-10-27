//! Сборка сервера на настоящих хранилищах: `routers` над PostgreSQL и `file://` blob.
//!
//! `CRANE_TEST_DATABASE_URL` - URL существующей БД: миграции идемпотентны, имя крейта
//! уникально на запуск, поэтому повторные прогоны на одной БД не мешают друг другу.
//! Без переменной тесты сообщают о пропуске, а при `CRANE_TEST_REQUIRE_DB=1` падают.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap - это падение теста"
)]

use std::collections::HashMap;
use std::io;
use std::path::Path;

use axum::Router;
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};
use std::str::FromStr;
use tokio::runtime::{Builder, Runtime};
use tower::ServiceExt;
use url::Url;
use uuid::Uuid;

use super::{migrate, routers};
use crate::config::Settings;

const URL_ENV: &str = "CRANE_TEST_DATABASE_URL";
const REQUIRE_ENV: &str = "CRANE_TEST_REQUIRE_DB";
const TOKEN: &str = "stack-token";
const CRATE: &[u8] = b"\x1f\x8b not really a gzip, but cargo does not look inside";

/// URL БД из окружения; без него тест пропускается (или падает при `CRANE_TEST_REQUIRE_DB=1`).
fn database_url() -> Option<String> {
    let url = std::env::var(URL_ENV).ok();
    if url.is_none() {
        assert!(
            std::env::var(REQUIRE_ENV).as_deref() != Ok("1"),
            "{REQUIRE_ENV}=1, но {URL_ENV} не задана: тесты с PostgreSQL нельзя пропускать"
        );
        report_skip();
    }
    url
}

#[allow(
    clippy::print_stderr,
    reason = "единственный способ сообщить о пропуске: тест при этом завершается успешно"
)]
fn report_skip() {
    eprintln!("ПРОПУСК: {URL_ENV} не задана, тесты с PostgreSQL не выполняются");
}

fn settings(database: &str, blobs_dir: &Path) -> Settings {
    let blobs = format!("file://{}", blobs_dir.display());
    let env: HashMap<&str, &str> = HashMap::from([
        ("CRANE_DATABASE_URL", database),
        ("CRANE_BLOBS", &blobs),
        ("CRANE_PUBLIC_URL", "https://crane.example"),
        ("CRANE_TOKEN", TOKEN),
    ]);
    let no_files =
        |_: &Path| -> io::Result<String> { Err(io::Error::from(io::ErrorKind::NotFound)) };
    Settings::from_env(&|name| env.get(name).map(|v| (*v).to_owned()), &no_files).unwrap()
}

/// Запущенный «сервер»: роутеры без сокетов. Миграции применяет роль `migrate`, как в compose.
struct Stack {
    runtime: Runtime,
    api: Router,
    admin: Router,
}

impl Stack {
    fn start(database: &str, blobs_dir: &Path, runtime: Runtime) -> Self {
        migrate(settings(database, blobs_dir).storage).unwrap();
        let (api, admin) = runtime
            .block_on(routers(settings(database, blobs_dir)))
            .unwrap();
        Self {
            runtime,
            api,
            admin,
        }
    }

    fn call(
        &self,
        router: &Router,
        method: Method,
        uri: &str,
        body: Vec<u8>,
    ) -> (StatusCode, Vec<u8>) {
        self.runtime.block_on(async {
            let request = Request::builder()
                .method(method)
                .uri(uri)
                .header("authorization", TOKEN)
                .body(Body::from(body))
                .unwrap();
            let response = router.clone().oneshot(request).await.unwrap();
            let status = response.status();
            let bytes = response.into_body().collect().await.unwrap().to_bytes();
            (status, bytes.to_vec())
        })
    }

    fn publish(&self, name: &str) -> StatusCode {
        self.call(
            &self.api,
            Method::PUT,
            "/cargo/internal/api/v1/crates/new",
            publish_body(name),
        )
        .0
    }

    /// Единственная строка индекса крейта как JSON.
    fn index_line(&self, name: &str) -> Value {
        let uri = format!("/cargo/internal/index/de/mo/{name}");
        let (_, body) = self.call(&self.api, Method::GET, &uri, Vec::new());
        serde_json::from_str(String::from_utf8(body).unwrap().trim()).unwrap()
    }
}

fn runtime() -> Runtime {
    Builder::new_current_thread().enable_all().build().unwrap()
}

/// Имя, уникальное на запуск: `demo-<hex>` (путь индекса `de/mo/…`).
fn unique_name() -> String {
    format!("demo-{}", &Uuid::new_v4().simple().to_string()[..12])
}

/// Тело `PUT /api/v1/crates/new`: длина JSON, JSON, длина крейта, крейт.
fn publish_body(name: &str) -> Vec<u8> {
    let meta = json!({
        "name": name, "vers": "1.2.3", "deps": [], "features": {},
        "description": "тестовый крейт", "license": "MIT"
    })
    .to_string();
    let mut out = Vec::new();
    out.extend_from_slice(&u32::try_from(meta.len()).unwrap().to_le_bytes());
    out.extend_from_slice(meta.as_bytes());
    out.extend_from_slice(&u32::try_from(CRATE.len()).unwrap().to_le_bytes());
    out.extend_from_slice(CRATE);
    out
}

#[test]
fn published_version_is_indexed_with_checksum_of_the_uploaded_crate() {
    // Arrange
    let Some(url) = database_url() else { return };
    let dir = tempfile::tempdir().unwrap();
    let stack = Stack::start(&url, dir.path(), runtime());
    let name = unique_name();
    stack.publish(&name);

    // Act
    let line = stack.index_line(&name);

    // Assert
    assert_eq!(line["cksum"], format!("{:x}", Sha256::digest(CRATE)));
    assert_eq!(line["yanked"], false);
}

#[test]
fn version_published_by_one_server_is_served_by_a_restarted_one() {
    // Arrange
    let Some(url) = database_url() else { return };
    let dir = tempfile::tempdir().unwrap();
    let name = unique_name();
    Stack::start(&url, dir.path(), runtime()).publish(&name);
    let restarted = Stack::start(&url, dir.path(), runtime());

    // Act
    let line = restarted.index_line(&name);

    // Assert
    assert_eq!(line["cksum"], format!("{:x}", Sha256::digest(CRATE)));
}

#[test]
fn ready_probe_is_ok_over_live_stores() {
    // Arrange
    let Some(url) = database_url() else { return };
    let dir = tempfile::tempdir().unwrap();
    let stack = Stack::start(&url, dir.path(), runtime());

    // Act
    let (status, _) = stack.call(&stack.admin, Method::GET, "/healthz/ready", Vec::new());

    // Assert
    assert_eq!(status, StatusCode::OK);
}

/// Пул к БД сервера из `CRANE_TEST_DATABASE_URL`, переключённый на базу `database`.
async fn pool_for(url: &str, database: &str) -> PgPool {
    let options = PgConnectOptions::from_str(url).unwrap().database(database);
    PgPoolOptions::new().connect_with(options).await.unwrap()
}

#[test]
fn migrate_creates_the_schema_in_an_empty_database() {
    // Arrange
    let Some(url) = database_url() else { return };
    let runtime = runtime();
    // Имя из hex uuid: идентификатор нельзя передать параметром, а такой безопасен.
    let name = format!("crane_migrate_{}", Uuid::new_v4().simple());
    // Администратор подключается к БД из URL как есть: она существует по определению.
    let admin = runtime.block_on(async {
        let options = PgConnectOptions::from_str(&url).unwrap();
        PgPoolOptions::new().connect_with(options).await.unwrap()
    });
    runtime
        .block_on(sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}"))).execute(&admin))
        .unwrap();
    let mut target = Url::parse(&url).unwrap();
    target.set_path(&format!("/{name}"));
    let dir = tempfile::tempdir().unwrap();
    let storage = settings(target.as_str(), dir.path()).storage;

    // Act
    let result = migrate(storage);

    // Assert
    // Если тест упал по panic, БД остаётся: чистка по префиксу убила бы БД параллельных процессов.
    let table: Option<String> = runtime.block_on(async {
        let pool = pool_for(&url, &name).await;
        let table = sqlx::query_scalar("SELECT to_regclass('package_version')::text")
            .fetch_one(&pool)
            .await
            .unwrap();
        pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {name} WITH (FORCE)"
        )))
        .execute(&admin)
        .await
        .unwrap();
        table
    });
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(table.as_deref(), Some("package_version"));
}
