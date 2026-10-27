//! Поведение, специфичное для PostgreSQL: атомарность, гонки, точность времени, миграции.

#[allow(
    dead_code,
    reason = "помощники общие для нескольких тестовых бинарников"
)]
mod support;

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crane_domain::registry::{FormatMetadata, VersionState};
use crane_pg::{PgCatalog, migrate};
use crane_ports::registry::{Catalog, CatalogError};
use support::{asset, hosted_repository, name, new_version, version};

const ROUNDS: u64 = 8;

#[test]
fn failed_first_publish_leaves_no_package_behind() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        let mut broken = new_version(10, repo, "Foo-Bar", "1.0.0");
        broken.assets = vec![asset("same.crate", 1, 1), asset("same.crate", 2, 2)];

        // Act
        let failed = catalog.publish_version(broken).await;
        let later = catalog
            .publish_version(new_version(11, repo, "foo_bar", "1.0.0"))
            .await;

        // Assert
        assert!(
            matches!(failed, Err(CatalogError::Backend(_))),
            "{failed:?}"
        );
        assert_eq!(
            later.map(|v| v.package.as_written().to_owned()).ok(),
            Some("foo_bar".to_owned())
        );
    });
}

#[test]
fn concurrent_duplicate_publish_yields_one_ok_and_version_exists() {
    support::run(async |server| {
        // Arrange
        let catalog = Arc::new(server.fresh_catalog().await);
        let repo = hosted_repository(&*catalog).await;

        for round in 0..ROUNDS {
            let package = format!("dup{round}");
            let (a, b) = (catalog.clone(), catalog.clone());
            let (first, second) = (
                new_version(100 + round * 2, repo, &package, "1.0.0"),
                new_version(101 + round * 2, repo, &package, "1.0.0"),
            );

            // Act
            let tasks = [
                tokio::spawn(async move { a.publish_version(first).await }),
                tokio::spawn(async move { b.publish_version(second).await }),
            ];
            let mut results = Vec::new();
            for task in tasks {
                results.push(task.await.expect("задача завершилась"));
            }

            // Assert
            let ok = results.iter().filter(|r| r.is_ok()).count();
            let exists = results
                .iter()
                .filter(|r| matches!(r, Err(CatalogError::VersionExists)))
                .count();
            assert_eq!((ok, exists), (1, 1), "раунд {round}: {results:?}");
        }
    });
}

#[test]
fn concurrent_first_publishes_under_different_spellings_share_one_package() {
    support::run(async |server| {
        // Arrange
        let catalog = Arc::new(server.fresh_catalog().await);
        let repo = hosted_repository(&*catalog).await;

        for round in 0..ROUNDS {
            let (a, b) = (catalog.clone(), catalog.clone());
            let dashed = new_version(200 + round * 2, repo, &format!("p{round}-x"), "1.0.0");
            let underscored = new_version(201 + round * 2, repo, &format!("P{round}_x"), "1.0.1");

            // Act
            let tasks = [
                tokio::spawn(async move { a.publish_version(dashed).await }),
                tokio::spawn(async move { b.publish_version(underscored).await }),
            ];
            let mut published = Vec::new();
            for task in tasks {
                published.push(task.await.expect("задача завершилась"));
            }

            // Assert
            let spellings: Vec<_> = published
                .iter()
                .map(|r| r.as_ref().map(|v| v.package.as_written().to_owned()).ok())
                .collect();
            let stored = catalog
                .versions(repo, &name(&format!("p{round}_x")))
                .await
                .expect("versions");
            assert!(
                spellings.iter().all(Option::is_some),
                "раунд {round}: {published:?}"
            );
            assert_eq!(spellings[0], spellings[1], "раунд {round}");
            assert_eq!(stored.len(), 2, "раунд {round}");
        }
    });
}

#[test]
fn publish_result_equals_find_result_for_nanosecond_timestamp() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        let mut input = new_version(1, repo, "nano", "1.0.0");
        input.published_at = SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_789);

        // Act
        let published = catalog.publish_version(input).await.expect("publish");
        let found = catalog
            .find_version(repo, &name("nano"), &version("1.0.0"))
            .await
            .expect("find");

        // Assert
        assert_eq!(
            published.published_at,
            SystemTime::UNIX_EPOCH + Duration::new(1_700_000_000, 123_456_000)
        );
        assert_eq!(found, Some(published));
    });
}

#[test]
fn migrate_is_idempotent_on_empty_database() {
    support::run(async |server| {
        // Arrange
        let pool = server.empty_pool().await;

        // Act
        let first = migrate(&pool).await;
        let second = migrate(&pool).await;

        // Assert
        let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("журнал миграций");
        assert!(first.is_ok() && second.is_ok(), "{first:?} {second:?}");
        assert_eq!(applied, 1);
    });
}

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
        let found = catalog
            .find_version(repo, &name("order"), &version("1.0.0"))
            .await
            .expect("find")
            .expect("версия есть");

        // Assert
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
        let catalog: Arc<PgCatalog> = Arc::new(server.fresh_catalog().await);
        let repo = hosted_repository(&*catalog).await;
        let id = catalog
            .publish_version(new_version(1, repo, "race", "1.0.0"))
            .await
            .expect("publish")
            .id;

        // Act
        let tasks: Vec<_> = (0..16)
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
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.expect("задача завершилась"));
        }

        // Assert
        let ok = results.iter().filter(|r| r.is_ok()).count();
        let conflicts = results
            .iter()
            .filter(|r| matches!(r, Err(CatalogError::StateConflict { .. })))
            .count();
        assert_eq!((ok, conflicts), (1, 15), "{results:?}");
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
        let found = catalog
            .find_version(repo, &name("bare"), &version("1.0.0"))
            .await
            .expect("find")
            .expect("версия есть");
        let listed = catalog
            .versions(repo, &name("bare"))
            .await
            .expect("versions");

        // Assert
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
        let found = catalog
            .find_version(repo, &name("twin"), &version("1.0.0"))
            .await
            .expect("find")
            .expect("версия есть");

        // Assert
        let expected = vec![asset("a.crate", 5, 9), asset("b.crate", 5, 9)];
        assert_eq!(published.assets, expected);
        assert_eq!(found.assets, expected);
    });
}
