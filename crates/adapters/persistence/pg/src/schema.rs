use sqlx::PgPool;
use sqlx::SqlSafeStr;
use sqlx::migrate::{MigrateError, Migration, MigrationType, Migrator};

/// Применяет недостающие миграции. Идемпотентно; параллельные запуски сериализует
/// advisory-блокировка `sqlx`, которую мы оставляем включённой.
pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    migrator().run(pool).await
}

// Миграции вшиваются в бинарник и регистрируются явно: макрос `sqlx::migrate!` в проекте
// не используется. Новая миграция - ещё одна строка в списке (см. migrations/README.md).
fn migrator() -> Migrator {
    Migrator::with_migrations(vec![Migration::new(
        1,
        "catalog".into(),
        MigrationType::Simple,
        include_str!("../migrations/0001_catalog.sql").into_sql_str(),
        false,
    )])
}
