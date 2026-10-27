use std::collections::BTreeSet;

use crane_domain::registry::{
    Format, PackageName, PackageVersion, Repository, RepositoryId, Version, VersionId, VersionState,
};
use crane_ports::registry::{Catalog, CatalogError, NewVersion};
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

mod rows;

use self::rows::{
    corrupt, kind_as_str, rejected, repository_from_row, state_as_str, state_parse, to_db_time,
    version_from_row,
};

/// Идентификатор хранилища blobs в таблице `blob`. Пока хранилище одно; когда появится
/// выбор, значение придёт из конфигурации, а не из схемы (у колонки нет default).
const STORE_ID: &str = "main";

/// Имена ограничений из миграции 0001: по ним нарушения превращаются в доменные ошибки.
const VERSION_KEY: &str = "package_version_package_id_version_key";
const PACKAGE_REPOSITORY_FKEY: &str = "package_repository_id_fkey";

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

/// Сбой драйвера или БД.
fn backend(error: sqlx::Error) -> CatalogError {
    CatalogError::Backend(Box::new(error))
}

/// Читает одну версию по id на соединении транзакции: так публикация возвращает ровно то,
/// что увидит `find_version`.
async fn load_version(
    conn: &mut PgConnection,
    id: VersionId,
) -> Result<Option<PackageVersion>, CatalogError> {
    sqlx::query(select_version!("WHERE v.id = $1"))
        .bind(id.as_uuid())
        .fetch_optional(conn)
        .await
        .map_err(backend)?
        .as_ref()
        .map(version_from_row)
        .transpose()
}

/// Ошибки `publish_version`: нарушения известных ограничений - доменные, остальное - сбой.
/// Любое другое нарушение уникальности (повтор пути файла, повтор id) - ошибка вызывающего
/// или повреждение, но не `VersionExists`: иначе клиент получил бы ложное «версия занята».
fn publish_error(error: sqlx::Error) -> CatalogError {
    if let Some(db) = error.as_database_error() {
        if db.is_unique_violation() && db.constraint() == Some(VERSION_KEY) {
            return CatalogError::VersionExists;
        }
        // Имя внешнего ключа однозначно: его может нести только его же нарушение,
        // поэтому отдельная проверка вида ошибки лишь добавила бы недостижимую ветку.
        if db.constraint() == Some(PACKAGE_REPOSITORY_FKEY) {
            return CatalogError::NotFound;
        }
    }
    backend(error)
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
        let published_at = to_db_time(new.published_at)?;
        let sizes = new
            .assets
            .iter()
            .map(|a| {
                i64::try_from(a.size).map_err(|_| rejected("размер файла не помещается в bigint"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let paths: Vec<&str> = new.assets.iter().map(|a| a.path.as_str()).collect();
        let digests: Vec<&[u8]> = new
            .assets
            .iter()
            .map(|a| a.sha256.as_bytes().as_slice())
            .collect();

        let mut tx = self.pool.begin().await.map_err(backend)?;

        // Пакет создаётся первой версией. DO NOTHING ждёт конкурента, вставившего ту же строку,
        // и после его коммита следующий SELECT видит запись: пакет один, написание - первое.
        sqlx::query(
            "INSERT INTO package (repository_id, name_norm, name_display) VALUES ($1, $2, $3)
         ON CONFLICT ON CONSTRAINT package_repository_id_name_norm_key DO NOTHING",
        )
        .bind(new.repository.as_uuid())
        .bind(new.package.normalized())
        .bind(new.package.as_written())
        .execute(&mut *tx)
        .await
        .map_err(publish_error)?;
        let package_id: i64 = sqlx::query_scalar(
            "SELECT id FROM package WHERE repository_id = $1 AND name_norm = $2",
        )
        .bind(new.repository.as_uuid())
        .bind(new.package.normalized())
        .fetch_one(&mut *tx)
        .await
        .map_err(backend)?;

        // Содержимое общее для версий и репозиториев: уже известный blob не трогаем.
        sqlx::query(
            "INSERT INTO blob (store_id, sha256, size)
         SELECT $1, t.digest, t.size FROM UNNEST($2::bytea[], $3::int8[]) AS t(digest, size)
         ON CONFLICT (store_id, sha256) DO NOTHING",
        )
        .bind(STORE_ID)
        .bind(&digests)
        .bind(&sizes)
        .execute(&mut *tx)
        .await
        .map_err(publish_error)?;

        sqlx::query(
            "INSERT INTO package_version
             (id, package_id, version, version_display, state, published_at, metadata)
         VALUES ($1, $2, $3, $4, $5::ver_state, $6, $7::json)",
        )
        .bind(new.id.as_uuid())
        .bind(package_id)
        .bind(new.version.canonical())
        .bind(new.version.as_written())
        .bind(state_as_str(new.state))
        .bind(published_at)
        .bind(new.metadata.as_str())
        .execute(&mut *tx)
        .await
        .map_err(publish_error)?;

        // Один запрос на все файлы вместо N обращений к БД.
        sqlx::query(
            "INSERT INTO asset (version_id, path, store_id, blob_sha256)
         SELECT $1, t.path, $2, t.digest FROM UNNEST($3::text[], $4::bytea[]) AS t(path, digest)",
        )
        .bind(new.id.as_uuid())
        .bind(STORE_ID)
        .bind(&paths)
        .bind(&digests)
        .execute(&mut *tx)
        .await
        .map_err(publish_error)?;

        // Теги переносятся в той же транзакции, что и версия: либо всё, либо ничего.
        // Дубли имён схлопываются заранее: один INSERT не может затронуть строку дважды.
        let tags: Vec<&str> = new
            .tags
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        sqlx::query(
            "INSERT INTO dist_tag (package_id, tag, version_id)
         SELECT $1, t.tag, $2 FROM UNNEST($3::text[]) AS t(tag)
         ON CONFLICT (package_id, tag) DO UPDATE SET version_id = EXCLUDED.version_id",
        )
        .bind(package_id)
        .bind(new.id.as_uuid())
        .bind(&tags)
        .execute(&mut *tx)
        .await
        .map_err(backend)?;

        // Результат читается из БД, а не собирается из входа: так publish и find совпадают
        // по построению (микросекунды времени, написание пакета, размер из blob, порядок путей).
        let stored = load_version(&mut tx, new.id)
            .await?
            .ok_or_else(|| corrupt("только что вставленная версия не читается"))?;
        tx.commit().await.map_err(backend)?;
        Ok(stored)
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
