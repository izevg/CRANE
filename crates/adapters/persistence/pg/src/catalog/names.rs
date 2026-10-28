//! Namespace и проверка «имя занято hosted-репозиторием»: запросы защиты от dependency confusion.

use crane_domain::registry::{Format, Namespace, NamespacePattern, PackageName};
use crane_ports::registry::CatalogError;
use sqlx::PgPool;

use super::backend;
use super::rows::{corrupt, namespace_from_row};

pub(super) async fn namespace_for(
    pool: &PgPool,
    format: Format,
    candidates: &[NamespacePattern],
) -> Result<Option<Namespace>, CatalogError> {
    // Паттерны сравниваются точно (`= ANY`), а не LIKE: `_` в LIKE - подстановочный символ.
    // Порядок кандидатов (специфичный первым) задаёт вызывающий, его сохраняет `array_position`.
    let patterns: Vec<String> = candidates.iter().map(|p| p.as_str().to_owned()).collect();
    sqlx::query(
        "SELECT workspace_id, format::text AS format, pattern FROM namespace
         WHERE format = $1::pkg_format AND pattern = ANY($2::text[])
         ORDER BY array_position($2::text[], pattern) LIMIT 1",
    )
    .bind(format.as_str())
    .bind(&patterns)
    .fetch_optional(pool)
    .await
    .map_err(backend)?
    .as_ref()
    .map(namespace_from_row)
    .transpose()
}

pub(super) async fn hosted_exists(
    pool: &PgPool,
    format: Format,
    name: &PackageName,
) -> Result<bool, CatalogError> {
    // Идёт по уникальному (repository_id, name_norm): репозиториев формата единицы.
    sqlx::query_scalar(
        "SELECT EXISTS (
             SELECT 1 FROM package p
             WHERE p.name_norm = $2
               AND p.repository_id IN (
                   SELECT id FROM repository WHERE format = $1::pkg_format AND kind = 'hosted'
               )
         )",
    )
    .bind(format.as_str())
    .bind(name.normalized())
    .fetch_one(pool)
    .await
    .map_err(backend)
}

pub(super) async fn ensure_namespace(
    pool: &PgPool,
    namespace: Namespace,
) -> Result<Namespace, CatalogError> {
    sqlx::query(
        "INSERT INTO namespace (workspace_id, format, pattern)
         VALUES ($1, $2::pkg_format, $3)
         ON CONFLICT ON CONSTRAINT namespace_format_pattern_key DO NOTHING",
    )
    .bind(namespace.workspace.as_uuid())
    .bind(namespace.format.as_str())
    .bind(namespace.pattern.as_str())
    .execute(pool)
    .await
    .map_err(backend)?;
    sqlx::query(
        "SELECT workspace_id, format::text AS format, pattern FROM namespace
         WHERE format = $1::pkg_format AND pattern = $2",
    )
    .bind(namespace.format.as_str())
    .bind(namespace.pattern.as_str())
    .fetch_optional(pool)
    .await
    .map_err(backend)?
    .as_ref()
    .map(namespace_from_row)
    .transpose()?
    .ok_or_else(|| corrupt("namespace не читается после вставки"))
}
