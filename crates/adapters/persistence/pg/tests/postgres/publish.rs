//! Публикация: атомарность, гонки пакета и версии, точность времени.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crane_ports::registry::{Catalog, CatalogError};

use crate::CONTENDERS;
use crate::support;
use crate::support::{asset, hosted_repository, join_all, name, new_version, version};

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
fn order_key_and_prerelease_flag_survive_a_round_trip() {
    support::run(async |server| {
        // Arrange
        let catalog = server.fresh_catalog().await;
        let repo = hosted_repository(&catalog).await;
        let published = new_version(300, repo, "rt", "1.2.3-rc.1");
        catalog
            .publish_version(published.clone())
            .await
            .expect("публикация");

        // Act
        let stored = catalog
            .find_version(repo, &name("rt"), &published.version)
            .await
            .expect("find_version")
            .expect("версия найдена");

        // Assert
        assert_eq!(
            (
                stored.version.order_key(),
                stored.version.is_prerelease(),
                stored.version.canonical(),
            ),
            (
                published.version.order_key(),
                true,
                published.version.canonical()
            )
        );
    });
}
