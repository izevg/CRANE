//! Миграции схемы: идемпотентность, расширения, права.

use crane_pg::{SchemaError, migrate};

use crate::support;

/// Число миграций в `migrator()`: два файла, `0001` и `0002`.
const MIGRATIONS: i64 = 3;

#[test]
fn migrate_is_idempotent_on_empty_database() {
    support::run(async |server| {
        // Arrange
        let pool = server.empty_pool().await;
        migrate(&pool).await.expect("первый запуск");

        // Act
        let second = migrate(&pool).await;

        // Assert
        let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("журнал миграций");
        assert!(second.is_ok(), "{second:?}");
        assert_eq!(applied, MIGRATIONS);
    });
}

#[test]
fn migrate_creates_required_extensions_on_empty_database() {
    support::run(async |server| {
        // Arrange
        let pool = server.empty_pool().await;

        // Act
        let result = migrate(&pool).await;

        // Assert
        let extensions: Vec<String> = sqlx::query_scalar(
            "SELECT extname::text FROM pg_extension WHERE extname IN ('vector', 'pg_trgm') ORDER BY 1",
        )
        .fetch_all(&pool)
        .await
        .expect("список расширений");
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(extensions, ["pg_trgm", "vector"]);
    });
}

#[test]
fn migrate_succeeds_without_create_rights_when_extensions_exist() {
    support::run(async |server| {
        // Arrange
        let (admin, role) = server.empty_pool_with_limited_role().await;
        for sql in ["CREATE EXTENSION vector", "CREATE EXTENSION pg_trgm"] {
            sqlx::query(sql).execute(&admin).await.expect("расширение");
        }

        // Act
        let result = migrate(&role).await;

        // Assert
        assert!(result.is_ok(), "{result:?}");
    });
}

#[test]
fn migrate_reports_forbidden_extension_and_applies_no_migrations() {
    support::run(async |server| {
        // Arrange
        let (_admin, role) = server.empty_pool_with_limited_role().await;

        // Act
        let result = migrate(&role).await;

        // Assert
        let journal: Option<String> =
            sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations')::text")
                .fetch_one(&role)
                .await
                .expect("журнал миграций");
        assert!(
            matches!(
                result,
                Err(SchemaError::ExtensionForbidden { name: "vector" })
            ),
            "{result:?}"
        );
        assert_eq!(journal, None);
    });
}

#[test]
fn migrate_reports_outdated_vector_extension() {
    support::run(async |server| {
        // Arrange
        let pool = server.empty_pool().await;
        for sql in [
            "CREATE EXTENSION vector",
            "UPDATE pg_extension SET extversion = '0.7.4' WHERE extname = 'vector'",
        ] {
            sqlx::query(sql).execute(&pool).await.expect("подготовка");
        }

        // Act
        let result = migrate(&pool).await;

        // Assert
        assert!(
            matches!(
                &result,
                Err(SchemaError::ExtensionOutdated { name: "vector", installed, minimum })
                    if installed == "0.7.4" && minimum == "0.8"
            ),
            "{result:?}"
        );
    });
}
