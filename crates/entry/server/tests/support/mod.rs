//! Свежая БД PostgreSQL для интеграционного теста сервера.
//!
//! `tests/support` соседнего крейта (`crane-pg`) отсюда не подключить, поэтому здесь
//! минимум той же конвенции: `CRANE_TEST_DATABASE_URL` - URL администратора сервера,
//! без неё тест пропускается, а при `CRANE_TEST_REQUIRE_DB=1` падает.

#![allow(
    clippy::expect_used,
    reason = "помощники тестов: сбой окружения должен ронять тест"
)]

use std::str::FromStr;

use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};
use uuid::Uuid;

const URL_ENV: &str = "CRANE_TEST_DATABASE_URL";
const REQUIRE_ENV: &str = "CRANE_TEST_REQUIRE_DB";

/// Выполняет `body` на свежей БД с применёнными миграциями и удаляет БД после теста.
pub(crate) fn run(body: impl AsyncFnOnce(PgPool)) {
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
        let options = PgConnectOptions::from_str(&url).expect("CRANE_TEST_DATABASE_URL валиден");
        let admin = PgPoolOptions::new()
            .max_connections(2)
            .connect_with(options.clone())
            .await
            .expect("подключение администратора");
        // Имя из hex uuid: идентификатор нельзя передать параметром, а такой безопасен.
        let name = format!("crane_srv_test_{}", Uuid::new_v4().simple());
        sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}")))
            .execute(&admin)
            .await
            .expect("создание БД");
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect_with(options.database(&name))
            .await
            .expect("подключение к тестовой БД");
        crane_pg::migrate(&pool).await.expect("миграции");

        body(pool.clone()).await;

        pool.close().await;
        sqlx::query(AssertSqlSafe(format!(
            "DROP DATABASE IF EXISTS {name} WITH (FORCE)"
        )))
        .execute(&admin)
        .await
        .expect("удаление БД");
        admin.close().await;
    });
}

#[allow(
    clippy::print_stderr,
    reason = "единственный способ сообщить о пропуске: тест при этом завершается успешно"
)]
fn report_skip() {
    eprintln!("ПРОПУСК: {URL_ENV} не задана, тесты с PostgreSQL не выполняются");
}
