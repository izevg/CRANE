//! Прикрепление файлов: параллельные загрузки колёс одной версии.

use std::sync::Arc;

use crane_domain::registry::{Asset, Sha256};
use crane_ports::registry::{Catalog, CatalogError};

use crate::CONTENDERS;
use crate::support;
use crate::support::{hosted_repository, join_all, name, new_version, version};

fn wheel(path: String, byte: u8) -> Asset {
    Asset {
        path,
        sha256: Sha256::from([byte; 32]),
        size: 3,
    }
}

#[test]
fn concurrent_attaches_of_different_paths_all_succeed_and_all_files_are_stored() {
    support::run(async |server| {
        // Arrange
        let catalog = Arc::new(server.fresh_catalog().await);
        let repo = hosted_repository(&*catalog).await;
        let id = catalog
            .publish_version(new_version(10, repo, "dup", "1.0.0"))
            .await
            .expect("публикация версии")
            .id;
        let tasks = (0..CONTENDERS)
            .map(|n| {
                let catalog = catalog.clone();
                // Часть колёс делит содержимое: общий blob вставляется конкурентно.
                let file = wheel(
                    format!("dup-1.0.0-{n}.whl"),
                    u8::try_from(n % 4).unwrap() + 20,
                );
                tokio::spawn(async move { catalog.add_asset(id, file).await })
            })
            .collect();

        // Act
        let results = join_all(tasks).await;

        // Assert
        let stored = catalog
            .find_version(repo, &name("dup"), &version("1.0.0"))
            .await
            .expect("find_version")
            .expect("версия есть");
        let failed = results.iter().filter(|r| r.is_err()).count();
        assert_eq!(
            (failed, stored.assets.len()),
            (0, usize::try_from(CONTENDERS).unwrap() + 1)
        );
    });
}

#[test]
fn concurrent_attaches_of_same_path_yield_one_ok_and_asset_exists() {
    support::run(async |server| {
        // Arrange
        let catalog = Arc::new(server.fresh_catalog().await);
        let repo = hosted_repository(&*catalog).await;
        let id = catalog
            .publish_version(new_version(10, repo, "dup", "1.0.0"))
            .await
            .expect("публикация версии")
            .id;
        let tasks = (0..CONTENDERS)
            .map(|n| {
                let catalog = catalog.clone();
                let file = wheel("dup-1.0.0.whl".to_owned(), u8::try_from(n).unwrap() + 20);
                tokio::spawn(async move { catalog.add_asset(id, file).await })
            })
            .collect();

        // Act
        let results = join_all(tasks).await;

        // Assert
        let ok = results.iter().filter(|r| r.is_ok()).count();
        let exists = results
            .iter()
            .filter(|r| matches!(r, Err(CatalogError::AssetExists)))
            .count();
        assert_eq!((ok, exists), (1, usize::try_from(CONTENDERS - 1).unwrap()));
    });
}
