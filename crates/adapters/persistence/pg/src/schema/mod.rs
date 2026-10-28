use std::fmt;

use sqlx::error::DatabaseError;
use sqlx::migrate::{MigrateError, Migration, MigrationType, Migrator};
use sqlx::{AssertSqlSafe, PgConnection, PgPool, SqlSafeStr};

/// Код SQLSTATE `insufficient_privilege`.
const INSUFFICIENT_PRIVILEGE: &str = "42501";

// Сериализует параллельные `migrate` на CREATE EXTENSION. Ключ не совпадает с ключом `sqlx`:
// тот выводится из имени БД, поэтому две блокировки не мешают друг другу.
const EXTENSIONS_LOCK_KEY: i64 = 0x4352_414E_4558_5400;

/// Версия расширения `major.minor`; порядок полей даёт сравнение по major, затем по minor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Version(u32, u32);

impl Version {
    /// Разбирает `major.minor[.patch][суффикс]` числами, а не строками (`0.10.0` новее `0.8`).
    /// Неразборчивая версия даёт `None`.
    fn parse(raw: &str) -> Option<Self> {
        let mut parts = raw.split('.');
        let major = leading_number(parts.next()?)?;
        let minor = leading_number(parts.next()?)?;
        Some(Self(major, minor))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.0, self.1)
    }
}

/// Число из начальных цифр части версии: `8rc1` даёт 8.
fn leading_number(part: &str) -> Option<u32> {
    let digits = part.len() - part.trim_start_matches(|c: char| c.is_ascii_digit()).len();
    part[..digits].parse().ok()
}

/// Расширение, без которого схема каталога не работает.
struct Requirement {
    name: &'static str,
    /// Без минимума важно лишь наличие расширения: версия не разбирается.
    minimum: Option<Version>,
}

const REQUIRED: [Requirement; 2] = [
    Requirement {
        name: "vector",
        minimum: Some(Version(0, 8)),
    },
    Requirement {
        name: "pg_trgm",
        minimum: None,
    },
];

/// Почему схема не подготовлена. Сообщения адресованы оператору: в каждом названо
/// расширение и сказано, что делать.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error(
        "расширение PostgreSQL {name} недоступно на сервере (или версия пакета ниже требуемой): \
         установите пакет с ним (pgvector >= 0.8 для vector, contrib для pg_trgm)"
    )]
    ExtensionUnavailable { name: &'static str },
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
        minimum: String,
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
    let Requirement { name, minimum } = *requirement;
    let installed: Option<String> =
        sqlx::query_scalar("SELECT extversion FROM pg_extension WHERE extname = $1")
            .bind(name)
            .fetch_optional(&mut *conn)
            .await
            .map_err(SchemaError::Extensions)?;
    if let Some(installed) = installed {
        return match minimum {
            Some(minimum) if !satisfies(&installed, minimum) => {
                Err(SchemaError::ExtensionOutdated {
                    name,
                    installed,
                    minimum: minimum.to_string(),
                })
            }
            _ => Ok(()),
        };
    }
    let available: Option<String> =
        sqlx::query_scalar("SELECT default_version FROM pg_available_extensions WHERE name = $1")
            .bind(name)
            .fetch_optional(&mut *conn)
            .await
            .map_err(SchemaError::Extensions)?
            .flatten();
    let usable = available.is_some_and(|version| minimum.is_none_or(|m| satisfies(&version, m)));
    if !usable {
        return Err(SchemaError::ExtensionUnavailable { name });
    }
    // Имя - константа из REQUIRED, а не ввод; идентификатор параметром не передать.
    sqlx::query(AssertSqlSafe(format!(
        "CREATE EXTENSION IF NOT EXISTS {name}"
    )))
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

/// Версия не ниже минимума; неразборчивая считается неподходящей (fail closed).
fn satisfies(version: &str, minimum: Version) -> bool {
    Version::parse(version).is_some_and(|version| version >= minimum)
}

// Миграции вшиваются в бинарник и регистрируются явно: макрос `sqlx::migrate!` в проекте
// не используется. Новая миграция - ещё одна строка в списке (см. migrations/README.md).
fn migrator() -> Migrator {
    Migrator::with_migrations(vec![
        Migration::new(
            1,
            "catalog".into(),
            MigrationType::Simple,
            include_str!("../../migrations/0001_catalog.sql").into_sql_str(),
            false,
        ),
        Migration::new(
            2,
            "npm".into(),
            MigrationType::Simple,
            include_str!("../../migrations/0002_npm.sql").into_sql_str(),
            false,
        ),
        Migration::new(
            3,
            "namespace".into(),
            MigrationType::Simple,
            include_str!("../../migrations/0003_namespace.sql").into_sql_str(),
            false,
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::{Version, satisfies};

    #[test]
    fn version_check_compares_numerically_and_rejects_garbage() {
        // Arrange
        let versions = [
            "0.8.0", "0.8", "0.7.4", "0.10.0", "1.0", "0.8.7", "", "x.y", "0",
        ];

        // Act
        let actual = versions.map(|version| satisfies(version, Version(0, 8)));

        // Assert
        assert_eq!(
            actual,
            [true, true, false, true, true, true, false, false, false]
        );
    }
}
