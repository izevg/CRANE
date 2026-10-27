//! Поведение, специфичное для PostgreSQL: атомарность, гонки, точность времени, миграции.

#[allow(
    dead_code,
    reason = "помощники общие для нескольких тестовых бинарников"
)]
mod support;

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crane_domain::registry::{FormatMetadata, VersionState};
use crane_pg::{SchemaError, migrate};
use crane_ports::registry::{Catalog, CatalogError};
use support::{asset, hosted_repository, join_all, name, new_version, version};

/// Число конкурентов в гонках: достаточно, чтобы окно между проверкой и записью пересеклось.
const CONTENDERS: u64 = 16;

#[test]
fn failed_first_publish_leaves_no_package_behind() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        let mut broken = new_version(10, repo, "Foo-Bar", "1.0.0");
        broken.assets = vec![asset("same.crate", 1, 1), asset("same.crate", 2, 2)];
        let failure = catalog
            .publish_version(broken)
            .await
            .expect_err("повтор пути файла отклоняется");

        // Act
        let later = catalog
            .publish_version(new_version(11, repo, "foo_bar", "1.0.0"))
            .await;

        // Assert
        assert!(matches!(failure, CatalogError::Backend(_)), "{failure:?}");
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
        let tasks = (0..CONTENDERS)
            .map(|n| {
                let catalog = catalog.clone();
                let contender = new_version(100 + n, repo, "dup", "1.0.0");
                tokio::spawn(async move { catalog.publish_version(contender).await })
            })
            .collect();

        // Act
        let results = join_all(tasks).await;

        // Assert
        let ok = results.iter().filter(|r| r.is_ok()).count();
        let exists = results
            .iter()
            .filter(|r| matches!(r, Err(CatalogError::VersionExists)))
            .count();
        assert_eq!((ok, exists), (1, usize::try_from(CONTENDERS - 1).unwrap()));
    });
}

#[test]
fn concurrent_first_publishes_under_different_spellings_share_one_package() {
    support::run(async |server| {
        // Arrange
        let catalog = Arc::new(server.fresh_catalog().await);
        let repo = hosted_repository(&*catalog).await;
        let tasks = (0..CONTENDERS)
            .map(|n| {
                let catalog = catalog.clone();
                let spelling = if n % 2 == 0 { "Foo-Bar" } else { "foo_bar" };
                let contender = new_version(200 + n, repo, spelling, &format!("1.0.{n}"));
                tokio::spawn(async move { catalog.publish_version(contender).await })
            })
            .collect();

        // Act
        let results = join_all(tasks).await;

        // Assert
        let stored = catalog
            .versions(repo, &name("foo_bar"))
            .await
            .expect("versions");
        let ok = results.iter().filter(|r| r.is_ok()).count();
        let spellings: BTreeSet<String> = results
            .iter()
            .filter_map(|r| r.as_ref().ok())
            .map(|v| v.package.as_written().to_owned())
            .collect();
        assert_eq!(
            (ok, spellings.len(), stored.len()),
            (
                usize::try_from(CONTENDERS).unwrap(),
                1,
                usize::try_from(CONTENDERS).unwrap()
            )
        );
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

        // Assert
        let found = catalog
            .find_version(repo, &name("nano"), &version("1.0.0"))
            .await
            .expect("find");
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
        migrate(&pool).await.expect("первый запуск");

        // Act
        let second = migrate(&pool).await;

        // Assert
        let applied: i64 = sqlx::query_scalar("SELECT count(*) FROM _sqlx_migrations")
            .fetch_one(&pool)
            .await
            .expect("журнал миграций");
        assert!(second.is_ok(), "{second:?}");
        assert_eq!(applied, 1);
    });
}

#[test]
fn migrate_creates_required_extensions_on_empty_database() {
    support::run(async |server| {
        // Arrange
        let pool = server.empty_pool().await;

        // Act
        let result = migrate(&pool).await;

        // Assert
        let extensions: Vec<String> = sqlx::query_scalar(
            "SELECT extname::text FROM pg_extension WHERE extname IN ('vector', 'pg_trgm') ORDER BY 1",
        )
        .fetch_all(&pool)
        .await
        .expect("список расширений");
        assert!(result.is_ok(), "{result:?}");
        assert_eq!(extensions, ["pg_trgm", "vector"]);
    });
}

#[test]
fn migrate_succeeds_without_create_rights_when_extensions_exist() {
    support::run(async |server| {
        // Arrange
        let (admin, role) = server.empty_pool_with_limited_role().await;
        for sql in ["CREATE EXTENSION vector", "CREATE EXTENSION pg_trgm"] {
            sqlx::query(sql).execute(&admin).await.expect("расширение");
        }

        // Act
        let result = migrate(&role).await;

        // Assert
        assert!(result.is_ok(), "{result:?}");
    });
}

#[test]
fn migrate_reports_forbidden_extension_and_applies_no_migrations() {
    support::run(async |server| {
        // Arrange
        let (_admin, role) = server.empty_pool_with_limited_role().await;

        // Act
        let result = migrate(&role).await;

        // Assert
        let journal: Option<String> =
            sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations')::text")
                .fetch_one(&role)
                .await
                .expect("журнал миграций");
        assert!(
            matches!(
                result,
                Err(SchemaError::ExtensionForbidden { name: "vector" })
            ),
            "{result:?}"
        );
        assert_eq!(journal, None);
    });
}

#[test]
fn migrate_reports_outdated_vector_extension() {
    support::run(async |server| {
        // Arrange
        let pool = server.empty_pool().await;
        for sql in [
            "CREATE EXTENSION vector",
            "UPDATE pg_extension SET extversion = '0.7.4' WHERE extname = 'vector'",
        ] {
            sqlx::query(sql).execute(&pool).await.expect("подготовка");
        }

        // Act
        let result = migrate(&pool).await;

        // Assert
        assert!(
            matches!(
                &result,
                Err(SchemaError::ExtensionOutdated { name: "vector", installed, minimum })
                    if installed == "0.7.4" && minimum == "0.8"
            ),
            "{result:?}"
        );
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
