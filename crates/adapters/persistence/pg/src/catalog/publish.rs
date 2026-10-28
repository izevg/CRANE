//! Публикация версии: шаги одной транзакции и сопоставление ошибок БД доменным.

use std::collections::BTreeSet;

use crane_domain::registry::{Asset, PackageVersion, VersionId};
use crane_ports::registry::{CatalogError, NewVersion};
use sqlx::{PgConnection, PgPool};

use super::backend;
use super::rows::{corrupt, rejected, state_as_str, to_db_time, version_from_row};

/// Идентификатор хранилища blobs в таблице `blob`. Пока хранилище одно; когда появится
/// выбор, значение придёт из конфигурации, а не из схемы (у колонки нет default).
const STORE_ID: &str = "main";

/// Имена ограничений из миграции 0001: по ним нарушения превращаются в доменные ошибки.
const VERSION_KEY: &str = "package_version_package_id_version_key";
const PACKAGE_REPOSITORY_FKEY: &str = "package_repository_id_fkey";
const ASSET_KEY: &str = "asset_pkey";
const ASSET_VERSION_FKEY: &str = "asset_version_id_fkey";

/// Файлы версии в виде параллельных массивов для `UNNEST`.
struct Files<'a> {
    paths: Vec<&'a str>,
    digests: Vec<&'a [u8]>,
    sizes: Vec<i64>,
}

/// Ошибки `publish_version` и `add_asset`: нарушения известных ограничений - доменные,
/// остальное - сбой. Любое другое нарушение уникальности (повтор id) - ошибка вызывающего
/// или повреждение, но не `VersionExists`: иначе клиент получил бы ложное «версия занята».
/// Повтор пути файла при публикации остаётся сбоем: пути в `NewVersion` задаёт вызывающий,
/// а `AssetExists` - ответ только на прикрепление к существующей версии.
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

/// Публикует версию одной транзакцией: пакет, blobs, версия, файлы, теги и чтение результата.
pub(super) async fn publish_version(
    pool: &PgPool,
    new: NewVersion,
) -> Result<PackageVersion, CatalogError> {
    let published_at = to_db_time(new.published_at)?;
    let files = files_of(&new.assets)?;

    let mut tx = pool.begin().await.map_err(backend)?;
    let package_id = ensure_package(&mut tx, &new).await?;
    insert_blobs(&mut tx, &files).await?;
    insert_version(&mut tx, &new, package_id, published_at).await?;
    insert_assets(&mut tx, new.id, &files)
        .await
        .map_err(publish_error)?;
    move_tags(&mut tx, &new, package_id).await?;

    // Результат читается из БД, а не собирается из входа: так publish и find совпадают
    // по построению (микросекунды времени, написание пакета, размер из blob, порядок путей).
    let stored = load_version(&mut tx, new.id)
        .await?
        .ok_or_else(|| corrupt("только что вставленная версия не читается"))?;
    tx.commit().await.map_err(backend)?;
    Ok(stored)
}

fn files_of(assets: &[Asset]) -> Result<Files<'_>, CatalogError> {
    let sizes = assets
        .iter()
        .map(|a| i64::try_from(a.size).map_err(|_| rejected("размер файла не помещается в bigint")))
        .collect::<Result<Vec<_>, _>>()?;
    let paths = assets.iter().map(|a| a.path.as_str()).collect();
    let digests = assets
        .iter()
        .map(|a| a.sha256.as_bytes().as_slice())
        .collect();
    Ok(Files {
        paths,
        digests,
        sizes,
    })
}

/// Пакет создаётся первой версией. DO NOTHING ждёт конкурента, вставившего ту же строку,
/// и после его коммита следующий SELECT видит запись: пакет один, написание - первое.
async fn ensure_package(conn: &mut PgConnection, new: &NewVersion) -> Result<i64, CatalogError> {
    sqlx::query(
        "INSERT INTO package (repository_id, name_norm, name_display) VALUES ($1, $2, $3)
         ON CONFLICT ON CONSTRAINT package_repository_id_name_norm_key DO NOTHING",
    )
    .bind(new.repository.as_uuid())
    .bind(new.package.normalized())
    .bind(new.package.as_written())
    .execute(&mut *conn)
    .await
    .map_err(publish_error)?;
    sqlx::query_scalar("SELECT id FROM package WHERE repository_id = $1 AND name_norm = $2")
        .bind(new.repository.as_uuid())
        .bind(new.package.normalized())
        .fetch_one(conn)
        .await
        .map_err(backend)
}

/// Содержимое общее для версий и репозиториев: уже известный blob не трогаем.
async fn insert_blobs(conn: &mut PgConnection, files: &Files<'_>) -> Result<(), CatalogError> {
    sqlx::query(
        "INSERT INTO blob (store_id, sha256, size)
         SELECT $1, t.digest, t.size FROM UNNEST($2::bytea[], $3::int8[]) AS t(digest, size)
         ON CONFLICT (store_id, sha256) DO NOTHING",
    )
    .bind(STORE_ID)
    .bind(&files.digests)
    .bind(&files.sizes)
    .execute(conn)
    .await
    .map_err(publish_error)?;
    Ok(())
}

async fn insert_version(
    conn: &mut PgConnection,
    new: &NewVersion,
    package_id: i64,
    published_at: time::OffsetDateTime,
) -> Result<(), CatalogError> {
    sqlx::query(
        "INSERT INTO package_version
             (id, package_id, version, version_display, version_order, prerelease, state,
              published_at, metadata)
         VALUES ($1, $2, $3, $4, $5, $6, $7::ver_state, $8, $9::json)",
    )
    .bind(new.id.as_uuid())
    .bind(package_id)
    .bind(new.version.canonical())
    .bind(new.version.as_written())
    .bind(new.version.order_key().as_bytes())
    .bind(new.version.is_prerelease())
    .bind(state_as_str(new.state))
    .bind(published_at)
    .bind(new.metadata.as_str())
    .execute(conn)
    .await
    .map_err(publish_error)?;
    Ok(())
}

/// Один запрос на все файлы вместо N обращений к БД.
async fn insert_assets(
    conn: &mut PgConnection,
    version: VersionId,
    files: &Files<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO asset (version_id, path, store_id, blob_sha256)
         SELECT $1, t.path, $2, t.digest FROM UNNEST($3::text[], $4::bytea[]) AS t(path, digest)",
    )
    .bind(version.as_uuid())
    .bind(STORE_ID)
    .bind(&files.paths)
    .bind(&files.digests)
    .execute(conn)
    .await?;
    Ok(())
}

/// Прикрепляет файл одной транзакцией: blob, строка файла и чтение результата.
/// Версию не блокируем: параллельные прикрепления разных путей не конфликтуют, а тот же путь
/// разрешает первичный ключ `(version_id, path)`.
pub(super) async fn add_asset(
    pool: &PgPool,
    version: VersionId,
    asset: Asset,
) -> Result<PackageVersion, CatalogError> {
    let files = files_of(std::slice::from_ref(&asset))?;

    let mut tx = pool.begin().await.map_err(backend)?;
    insert_blobs(&mut tx, &files).await?;
    insert_assets(&mut tx, version, &files)
        .await
        .map_err(asset_error)?;
    let stored = load_version(&mut tx, version)
        .await?
        .ok_or_else(|| corrupt("версия с только что вставленным файлом не читается"))?;
    tx.commit().await.map_err(backend)?;
    Ok(stored)
}

/// Занятый путь и неизвестная версия - доменные ответы `add_asset`, остальное - сбой.
fn asset_error(error: sqlx::Error) -> CatalogError {
    if let Some(db) = error.as_database_error() {
        // Имя ограничения однозначно (как у `PACKAGE_REPOSITORY_FKEY`): отдельная проверка
        // вида нарушения добавила бы недостижимую ветку.
        match db.constraint() {
            Some(ASSET_KEY) => return CatalogError::AssetExists,
            Some(ASSET_VERSION_FKEY) => return CatalogError::NotFound,
            _ => {}
        }
    }
    backend(error)
}

/// Теги переносятся в той же транзакции, что и версия: либо всё, либо ничего.
/// Дубли имён схлопываются заранее: один INSERT не может затронуть строку дважды.
async fn move_tags(
    conn: &mut PgConnection,
    new: &NewVersion,
    package_id: i64,
) -> Result<(), CatalogError> {
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
    .execute(conn)
    .await
    .map_err(backend)?;
    Ok(())
}
