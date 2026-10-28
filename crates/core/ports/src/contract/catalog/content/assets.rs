use crane_domain::registry::{Asset, PackageVersion, RepositoryId, Sha256, VersionId};

use crate::contract::catalog::{
    ensure_repo, find, new_version, publish, publish_deleted, unknown_version_id,
};
use crate::registry::{Catalog, CatalogError};

/// Сценарии группы; порядок вызовов сохраняет порядок общего прогона.
pub(super) async fn scenarios<C: Catalog>(new: &impl Fn() -> C) {
    add_asset_returns_all_files_sorted_by_path(&new()).await;
    add_asset_is_visible_to_find_version_as_returned(&new()).await;
    add_asset_with_occupied_path_is_asset_exists_and_changes_nothing(&new()).await;
    add_asset_with_occupied_path_and_same_digest_is_asset_exists(&new()).await;
    add_asset_with_path_used_by_other_version_succeeds(&new()).await;
    add_asset_with_known_blob_succeeds(&new()).await;
    add_asset_to_unknown_version_is_not_found(&new()).await;
    add_asset_to_deleted_version_succeeds(&new()).await;
}

fn asset(path: &str, byte: u8) -> Asset {
    Asset {
        path: path.to_owned(),
        sha256: Sha256::from([byte; 32]),
        size: 3,
    }
}

fn paths(assets: &[Asset]) -> Vec<&str> {
    assets.iter().map(|a| a.path.as_str()).collect()
}

/// Публикует `serde 1.0.0` с файлом `serde-1.0.0.crate` и возвращает его id.
async fn published<C: Catalog>(catalog: &C, repo: RepositoryId) -> VersionId {
    publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id
}

async fn find_serde<C: Catalog>(catalog: &C, repo: RepositoryId) -> PackageVersion {
    find(catalog, repo, "serde", "1.0.0").await
}

async fn add_asset_returns_all_files_sorted_by_path<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let id = published(catalog, repo).await;
    catalog
        .add_asset(id, asset("z.whl", 1))
        .await
        .expect("первый add_asset");

    // Act
    let result = catalog.add_asset(id, asset("a.whl", 2)).await;

    // Assert
    let record = result.expect("add_asset");
    assert_eq!(
        paths(&record.assets),
        vec!["a.whl", "serde-1.0.0.crate", "z.whl"],
        "add_asset отдаёт файлы по возрастанию пути в байтовом порядке, как find_version"
    );
}

async fn add_asset_is_visible_to_find_version_as_returned<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let id = published(catalog, repo).await;
    let returned = catalog
        .add_asset(id, asset("b.whl", 2))
        .await
        .expect("add_asset");

    // Act
    let found = find_serde(catalog, repo).await;

    // Assert
    assert_eq!(
        found, returned,
        "add_asset возвращает ровно то, что потом увидит find_version"
    );
}

async fn add_asset_with_occupied_path_is_asset_exists_and_changes_nothing<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let id = published(catalog, repo).await;
    let before = find_serde(catalog, repo).await;

    // Act
    let result = catalog.add_asset(id, asset("serde-1.0.0.crate", 9)).await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::AssetExists)),
        "занятый путь даёт AssetExists, получили {result:?}"
    );
    assert_eq!(
        find_serde(catalog, repo).await,
        before,
        "отказ не меняет файлы версии"
    );
}

async fn add_asset_with_occupied_path_and_same_digest_is_asset_exists<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let id = published(catalog, repo).await;

    // Act
    let result = catalog.add_asset(id, asset("serde-1.0.0.crate", 7)).await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::AssetExists)),
        "порт не идемпотентен: повтор того же файла решает use case, получили {result:?}"
    );
}

async fn add_asset_with_path_used_by_other_version_succeeds<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    published(catalog, repo).await;
    let other = publish(catalog, new_version(11, repo, "serde", "1.0.1"))
        .await
        .id;

    // Act
    let result = catalog
        .add_asset(other, asset("serde-1.0.0.crate", 3))
        .await;

    // Assert
    assert!(
        result.is_ok(),
        "путь уникален внутри версии, а не пакета: {result:?}"
    );
}

async fn add_asset_with_known_blob_succeeds<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let id = published(catalog, repo).await;

    // Act
    let result = catalog.add_asset(id, asset("copy.crate", 7)).await;

    // Assert
    let record = result.expect("add_asset");
    assert_eq!(
        paths(&record.assets),
        vec!["copy.crate", "serde-1.0.0.crate"],
        "содержимое общее для файлов: уже известный blob не мешает прикреплению"
    );
}

async fn add_asset_to_unknown_version_is_not_found<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_repo(catalog, 1, "main").await;

    // Act
    let result = catalog
        .add_asset(unknown_version_id(), asset("a.whl", 1))
        .await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::NotFound)),
        "неизвестный id даёт NotFound, получили {result:?}"
    );
}

async fn add_asset_to_deleted_version_succeeds<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish_deleted(catalog, repo).await;
    let id = find_serde(catalog, repo).await.id;

    // Act
    let result = catalog.add_asset(id, asset("late.whl", 4)).await;

    // Assert
    assert!(
        result.is_ok(),
        "состояние версии порт не проверяет, это правило use case: {result:?}"
    );
}
