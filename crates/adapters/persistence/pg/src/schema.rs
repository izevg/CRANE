use sqlx::error::DatabaseError;
use sqlx::migrate::{MigrateError, Migration, MigrationType, Migrator};
use sqlx::{PgConnection, PgPool, SqlSafeStr};

/// Код SQLSTATE `insufficient_privilege`.
const INSUFFICIENT_PRIVILEGE: &str = "42501";

// Сериализует параллельные `migrate` на CREATE EXTENSION. Ключ не совпадает с ключом `sqlx`:
// тот выводится из имени БД, поэтому две блокировки не мешают друг другу.
const EXTENSIONS_LOCK_KEY: i64 = 0x4352_414E_4558_5400;

/// Расширение, без которого схема каталога не работает.
struct Requirement {
    name: &'static str,
    /// Минимальная версия для сравнения и для текста ошибки.
    minimum: (u32, u32),
    minimum_text: &'static str,
    /// Имя расширения здесь константа, а не ввод: идентификатор нельзя передать параметром.
    create: &'static str,
}

const REQUIRED: [Requirement; 2] = [
    Requirement {
        name: "vector",
        minimum: (0, 8),
        minimum_text: "0.8",
        create: "CREATE EXTENSION IF NOT EXISTS vector",
    },
    Requirement {
        name: "pg_trgm",
        minimum: (0, 0),
        minimum_text: "0",
        create: "CREATE EXTENSION IF NOT EXISTS pg_trgm",
    },
];

/// Почему схема не подготовлена. Сообщения адресованы оператору: в каждом названо
/// расширение и сказано, что делать.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error(
        "расширение PostgreSQL {name} (версия не ниже {minimum}) недоступно на сервере: \
         установите пакет с ним (pgvector для vector, contrib для pg_trgm)"
    )]
    ExtensionUnavailable {
        name: &'static str,
        minimum: &'static str,
    },
    #[error(
        "расширения PostgreSQL {name} нет, а роли не хватает прав его создать: администратор БД \
         выполняет в базе каталога `CREATE EXTENSION {name};`"
    )]
    ExtensionForbidden { name: &'static str },
    #[error(
        "расширение PostgreSQL {name} версии {installed} старше требуемой {minimum}: администратор \
         БД выполняет в базе каталога `ALTER EXTENSION {name} UPDATE;`"
    )]
    ExtensionOutdated {
        name: &'static str,
        installed: String,
        minimum: &'static str,
    },
    #[error("проверка расширений PostgreSQL: {0}")]
    Extensions(#[source] sqlx::Error),
    #[error(transparent)]
    Migrate(#[from] MigrateError),
}

/// Гарантирует расширения `vector` (>= 0.8) и `pg_trgm`, затем применяет недостающие миграции.
/// Идемпотентно; параллельные запуски сериализуют блокировка расширений и advisory-блокировка
/// `sqlx`, которую мы оставляем включённой. Без расширений миграции не применяются.
pub async fn migrate(pool: &PgPool) -> Result<(), SchemaError> {
    ensure_extensions(pool).await?;
    migrator().run(pool).await?;
    Ok(())
}

async fn ensure_extensions(pool: &PgPool) -> Result<(), SchemaError> {
    let mut tx = pool.begin().await.map_err(SchemaError::Extensions)?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(EXTENSIONS_LOCK_KEY)
        .execute(&mut *tx)
        .await
        .map_err(SchemaError::Extensions)?;
    for requirement in &REQUIRED {
        ensure_extension(&mut tx, requirement).await?;
    }
    // Ошибка выше роняет транзакцию без commit: drop откатывает её.
    tx.commit().await.map_err(SchemaError::Extensions)
}

async fn ensure_extension(
    conn: &mut PgConnection,
    requirement: &Requirement,
) -> Result<(), SchemaError> {
    let Requirement {
        name,
        minimum,
        minimum_text,
        create,
    } = *requirement;
    let installed: Option<String> =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = $1")
            .bind(name)
            .fetch_optional(&mut *conn)
            .await
            .map_err(SchemaError::Extensions)?;
    if let Some(installed) = installed {
        return if at_least(&installed, minimum) {
            Ok(())
        } else {
            Err(SchemaError::ExtensionOutdated {
                name,
                installed,
                minimum: minimum_text,
            })
        };
    }
    let available: Option<String> =
        sqlx::query_scalar("SELECT default_version FROM pg_available_extensions WHERE name = $1")
            .bind(name)
            .fetch_optional(&mut *conn)
            .await
            .map_err(SchemaError::Extensions)?
            .flatten();
    if !available.is_some_and(|version| at_least(&version, minimum)) {
        return Err(SchemaError::ExtensionUnavailable {
            name,
            minimum: minimum_text,
        });
    }
    sqlx::query(create)
        .execute(&mut *conn)
        .await
        .map(drop)
        .map_err(|error| {
            let forbidden = error
                .as_database_error()
                .and_then(DatabaseError::code)
                .is_some_and(|code| code == INSUFFICIENT_PRIVILEGE);
            if forbidden {
                SchemaError::ExtensionForbidden { name }
            } else {
                SchemaError::Extensions(error)
            }
        })
}

/// Сравнивает `major.minor[.patch][суффикс]` с минимумом числами, а не строками
/// (`0.10.0` новее `0.8`). Неразборчивая версия считается неподходящей (fail closed).
fn at_least(version: &str, minimum: (u32, u32)) -> bool {
    let mut parts = version.split('.');
    let (Some(major), Some(minor)) = (
        parts.next().and_then(leading_number),
        parts.next().and_then(leading_number),
    ) else {
        return false;
    };
    (major, minor) >= minimum
}

/// Число из начальных цифр части версии: `8rc1` даёт 8.
fn leading_number(part: &str) -> Option<u32> {
    let digits = part.len() - part.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    part[..digits].parse().ok()
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

#[cfg(test)]
mod tests {
    use super::at_least;

    #[test]
    fn at_least_compares_versions_numerically_and_rejects_garbage() {
        // Arrange
        let versions = [
            "0.8.0", "0.8", "0.7.4", "0.10.0", "1.0", "0.8.7", "", "x.y", "0",
        ];

        // Act
        let actual = versions.map(|version| at_least(version, (0, 8)));

        // Assert
        assert_eq!(
            actual,
            [true, true, false, true, true, true, false, false, false]
        );
    }
}
