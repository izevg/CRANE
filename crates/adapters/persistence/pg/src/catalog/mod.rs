use crane_domain::registry::{
    Format, PackageName, PackageVersion, Repository, RepositoryId, Version, VersionId, VersionState,
};
use crane_ports::registry::{Catalog, CatalogError, NewVersion};
use sqlx::PgPool;
use uuid::Uuid;

/// Выборка версии со всеми файлами; хвост (`WHERE`/`ORDER BY`) добавляет каждый запрос.
/// Файлы читает один коррелированный LATERAL: `asset` и `blob` проходятся по версии один раз,
/// а не тремя подзапросами-массивами. Порядок задан в каждом `array_agg` одним и тем же
/// выражением: `COLLATE "C"` даёт байтовый порядок путей, одинаковый для любой локали
/// кластера и совпадающий с `Ord` для `String`. Размер берётся из `blob`, а не хранится
/// в `asset`: источник истины один. Без файлов `array_agg` даёт NULL, поэтому `COALESCE`.
macro_rules! select_version {
    ($tail:literal) => {
        concat!(
            r#"SELECT r.format::text AS format, p.name_display, v.id, r.id AS repository_id,
                v.version_display, v.state::text AS state, v.yanked, v.deprecated, v.published_at,
                v.metadata::text AS metadata,
                COALESCE(f.paths, '{}') AS paths,
                COALESCE(f.digests, '{}') AS digests,
                COALESCE(f.sizes, '{}') AS sizes
               FROM package_version v
               JOIN package p ON p.id = v.package_id
               JOIN repository r ON r.id = p.repository_id
               LEFT JOIN LATERAL (
                   SELECT array_agg(a.path ORDER BY a.path COLLATE "C") AS paths,
                          array_agg(a.blob_sha256 ORDER BY a.path COLLATE "C") AS digests,
                          array_agg(b.size ORDER BY a.path COLLATE "C") AS sizes
                   FROM asset a
                   JOIN blob b ON b.store_id = a.store_id AND b.sha256 = a.blob_sha256
                   WHERE a.version_id = v.id
               ) f ON true "#,
            $tail
        )
    };
}

mod publish;
mod rows;

use self::rows::{
    backend, corrupt, kind_as_str, repository_from_row, state_as_str, state_parse, version_from_row,
};

/// Каталог реестра в PostgreSQL. Схему создаёт [`crate::migrate`].
#[derive(Debug)]
pub struct PgCatalog {
    pool: PgPool,
}

impl PgCatalog {
    #[must_use]
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl Catalog for PgCatalog {
    async fn ensure_repository(&self, repository: Repository) -> Result<Repository, CatalogError> {
        sqlx::query(
            "INSERT INTO repository (id, format, kind, name)
             VALUES ($1, $2::pkg_format, $3::repo_kind, $4)
             ON CONFLICT ON CONSTRAINT repository_format_name_key DO NOTHING",
        )
        .bind(repository.id.as_uuid())
        .bind(repository.format.as_str())
        .bind(kind_as_str(repository.kind))
        .bind(&repository.name)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        self.find_repository(repository.format, &repository.name)
            .await?
            .ok_or_else(|| corrupt("репозиторий не читается после вставки"))
    }

    async fn find_repository(
        &self,
        format: Format,
        name: &str,
    ) -> Result<Option<Repository>, CatalogError> {
        sqlx::query(
            "SELECT id, format::text AS format, kind::text AS kind, name FROM repository
             WHERE format = $1::pkg_format AND name = $2",
        )
        .bind(format.as_str())
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?
        .as_ref()
        .map(repository_from_row)
        .transpose()
    }

    async fn publish_version(&self, new: NewVersion) -> Result<PackageVersion, CatalogError> {
        publish::publish_version(&self.pool, new).await
    }

    async fn versions(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> Result<Vec<PackageVersion>, CatalogError> {
        let rows = sqlx::query(select_version!(
            "WHERE p.repository_id = $1 AND p.name_norm = $2 ORDER BY v.published_at, v.id"
        ))
        .bind(repository.as_uuid())
        .bind(package.normalized())
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        rows.iter().map(version_from_row).collect()
    }

    async fn find_version(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        version: &Version,
    ) -> Result<Option<PackageVersion>, CatalogError> {
        sqlx::query(select_version!(
            "WHERE p.repository_id = $1 AND p.name_norm = $2 AND v.version = $3"
        ))
        .bind(repository.as_uuid())
        .bind(package.normalized())
        .bind(version.canonical())
        .fetch_optional(&self.pool)
        .await
        .map_err(backend)?
        .as_ref()
        .map(version_from_row)
        .transpose()
    }

    async fn set_yanked(&self, id: VersionId, yanked: bool) -> Result<(), CatalogError> {
        let done = sqlx::query("UPDATE package_version SET yanked = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(yanked)
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        if done.rows_affected() == 0 {
            return Err(CatalogError::NotFound);
        }
        Ok(())
    }

    async fn set_deprecated(
        &self,
        id: VersionId,
        message: Option<String>,
    ) -> Result<(), CatalogError> {
        let done = sqlx::query("UPDATE package_version SET deprecated = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(message)
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        if done.rows_affected() == 0 {
            return Err(CatalogError::NotFound);
        }
        Ok(())
    }

    async fn set_tag(&self, version: VersionId, tag: &str) -> Result<(), CatalogError> {
        // Пакет берётся из самой версии: тег чужого пакета выразить нельзя.
        let done = sqlx::query(
            "INSERT INTO dist_tag (package_id, tag, version_id)
             SELECT package_id, $2, id FROM package_version WHERE id = $1
             ON CONFLICT (package_id, tag) DO UPDATE SET version_id = EXCLUDED.version_id",
        )
        .bind(version.as_uuid())
        .bind(tag)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        if done.rows_affected() == 0 {
            return Err(CatalogError::NotFound);
        }
        Ok(())
    }

    async fn remove_tag(
        &self,
        repository: RepositoryId,
        package: &PackageName,
        tag: &str,
    ) -> Result<bool, CatalogError> {
        let done = sqlx::query(
            "DELETE FROM dist_tag d USING package p
             WHERE d.package_id = p.id AND p.repository_id = $1 AND p.name_norm = $2 AND d.tag = $3",
        )
        .bind(repository.as_uuid())
        .bind(package.normalized())
        .bind(tag)
        .execute(&self.pool)
        .await
        .map_err(backend)?;
        Ok(done.rows_affected() > 0)
    }

    async fn tags(
        &self,
        repository: RepositoryId,
        package: &PackageName,
    ) -> Result<Vec<(String, VersionId)>, CatalogError> {
        // COLLATE "C": байтовый порядок, одинаковый для любой локали кластера и совпадающий с Ord для String.
        let rows: Vec<(String, Uuid)> = sqlx::query_as(
            "SELECT d.tag, d.version_id FROM dist_tag d JOIN package p ON p.id = d.package_id
             WHERE p.repository_id = $1 AND p.name_norm = $2 ORDER BY d.tag COLLATE \"C\"",
        )
        .bind(repository.as_uuid())
        .bind(package.normalized())
        .fetch_all(&self.pool)
        .await
        .map_err(backend)?;
        Ok(rows
            .into_iter()
            .map(|(tag, id)| (tag, VersionId::new(id)))
            .collect())
    }

    async fn transition(
        &self,
        id: VersionId,
        from: VersionState,
        to: VersionState,
    ) -> Result<(), CatalogError> {
        let mut tx = self.pool.begin().await.map_err(backend)?;
        // FOR UPDATE превращает сравнение с `from` и запись в одно атомарное действие:
        // из двух конкурентных переходов второй увидит уже новое состояние.
        let current: Option<String> =
            sqlx::query_scalar("SELECT state::text FROM package_version WHERE id = $1 FOR UPDATE")
                .bind(id.as_uuid())
                .fetch_optional(&mut *tx)
                .await
                .map_err(backend)?;
        let actual = state_parse(&current.ok_or(CatalogError::NotFound)?)?;
        if actual != from {
            return Err(CatalogError::StateConflict { actual });
        }
        sqlx::query("UPDATE package_version SET state = $2::ver_state WHERE id = $1")
            .bind(id.as_uuid())
            .bind(state_as_str(to))
            .execute(&mut *tx)
            .await
            .map_err(backend)?;
        tx.commit().await.map_err(backend)
    }
}
