//! Чтение версии: порядок файлов и версий, метаданные, переходы состояния.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crane_domain::registry::{FormatMetadata, VersionState};
use crane_ports::registry::{Catalog, CatalogError};

use crate::CONTENDERS;
use crate::support;
use crate::support::{asset, hosted_repository, join_all, name, new_version, version};

#[test]
fn assets_come_back_in_byte_order_of_path_regardless_of_input_order() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        let mut input = new_version(1, repo, "order", "1.0.0");
        input.assets = vec![
            asset("z.crate", 3, 30),
            asset("a.crate", 2, 20),
            asset("B.crate", 1, 10),
        ];

        // Act
        let published = catalog.publish_version(input).await.expect("publish");

        // Assert
        let found = catalog
            .find_version(repo, &name("order"), &version("1.0.0"))
            .await
            .expect("find")
            .expect("версия есть");
        let expected = vec![
            asset("B.crate", 1, 10),
            asset("a.crate", 2, 20),
            asset("z.crate", 3, 30),
        ];
        assert_eq!(published.assets, expected);
        assert_eq!(found.assets, expected);
    });
}

#[test]
fn metadata_is_returned_byte_for_byte() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        let raw = "{\"b\":   1,\n \"a\": [ ],\"b\":2}";
        let mut input = new_version(1, repo, "meta", "1.0.0");
        input.metadata = FormatMetadata::new(raw.to_owned());

        // Act
        let published = catalog.publish_version(input).await.expect("publish");

        // Assert
        assert_eq!(published.metadata.as_str(), raw);
    });
}

#[test]
fn concurrent_transitions_from_same_state_let_exactly_one_win() {
    support::run(async |server| {
        // Arrange
        let catalog = Arc::new(server.fresh_catalog().await);
        let repo = hosted_repository(&*catalog).await;
        let id = catalog
            .publish_version(new_version(1, repo, "race", "1.0.0"))
            .await
            .expect("publish")
            .id;
        let tasks = (0..CONTENDERS)
            .map(|n| {
                let catalog = catalog.clone();
                let to = if n % 2 == 0 {
                    VersionState::Released
                } else {
                    VersionState::Blocked
                };
                tokio::spawn(
                    async move { catalog.transition(id, VersionState::Quarantined, to).await },
                )
            })
            .collect();

        // Act
        let results = join_all(tasks).await;

        // Assert
        let ok = results.iter().filter(|r| r.is_ok()).count();
        let conflicts = results
            .iter()
            .filter(|r| matches!(r, Err(CatalogError::StateConflict { .. })))
            .count();
        assert_eq!(
            (ok, conflicts),
            (1, usize::try_from(CONTENDERS - 1).unwrap())
        );
    });
}

#[test]
fn versions_are_ordered_by_publication_time_then_id_not_by_insertion() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        // Вставка идёт в порядке 1.0.0, 1.0.1, 1.0.2, 1.0.3, а время публикации и id - иные.
        let plan = [
            ("1.0.0", 900, 30),
            ("1.0.1", 800, 20),
            ("1.0.2", 700, 20),
            ("1.0.3", 600, 10),
        ];
        for (raw, id, secs) in plan {
            let mut input = new_version(id, repo, "ord", raw);
            input.published_at = SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
            catalog.publish_version(input).await.expect("publish");
        }

        // Act
        let all = catalog
            .versions(repo, &name("ord"))
            .await
            .expect("versions");

        // Assert
        let written: Vec<&str> = all.iter().map(|v| v.version.as_written()).collect();
        assert_eq!(written, ["1.0.3", "1.0.2", "1.0.1", "1.0.0"]);
    });
}

#[test]
fn version_without_assets_has_empty_asset_list_everywhere() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        let mut input = new_version(1, repo, "bare", "1.0.0");
        input.assets = Vec::new();

        // Act
        let published = catalog.publish_version(input).await.expect("publish");

        // Assert
        let found = catalog
            .find_version(repo, &name("bare"), &version("1.0.0"))
            .await
            .expect("find")
            .expect("версия есть");
        let listed = catalog
            .versions(repo, &name("bare"))
            .await
            .expect("versions");
        let listed_counts: Vec<usize> = listed.iter().map(|v| v.assets.len()).collect();
        assert_eq!(
            (published.assets.len(), found.assets.len(), listed_counts),
            (0, 0, vec![0])
        );
    });
}

#[test]
fn two_assets_with_identical_content_are_both_stored() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        let mut input = new_version(1, repo, "twin", "1.0.0");
        input.assets = vec![asset("a.crate", 5, 9), asset("b.crate", 5, 9)];

        // Act
        let published = catalog.publish_version(input).await.expect("publish");

        // Assert
        let found = catalog
            .find_version(repo, &name("twin"), &version("1.0.0"))
            .await
            .expect("find")
            .expect("версия есть");
        let expected = vec![asset("a.crate", 5, 9), asset("b.crate", 5, 9)];
        assert_eq!(published.assets, expected);
        assert_eq!(found.assets, expected);
    });
}

#[test]
fn asset_size_beyond_bigint_is_rejected_without_side_effects() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        let mut input = new_version(1, repo, "huge", "1.0.0");
        input.assets = vec![asset("huge.crate", 1, u64::MAX)];

        // Act
        let result = catalog.publish_version(input).await;

        // Assert
        let stored = catalog
            .find_version(repo, &name("huge"), &version("1.0.0"))
            .await
            .expect("find");
        assert!(
            matches!(result, Err(CatalogError::Backend(_))),
            "{result:?}"
        );
        assert_eq!(stored, None);
    });
}
