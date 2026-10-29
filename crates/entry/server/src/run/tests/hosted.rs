#![allow(
    clippy::unwrap_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use std::future::Future;

use crane_domain::registry::Format;
use crane_memory::{MemoryCatalog, SequentialIdGen};
use tokio::runtime::Builder;

use super::*;

const NAME: &str = "web";

fn block_on<T>(future: impl Future<Output = T>) -> T {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

fn configured() -> Repos {
    Repos::of(Format::Npm, RepositoryKind::Hosted, vec![NAME.to_owned()])
}

async fn stored_id(catalog: &MemoryCatalog, format: Format) -> Option<RepositoryId> {
    catalog
        .find_repository(format, NAME)
        .await
        .unwrap()
        .map(|repository| repository.id)
}

#[test]
fn configured_hosted_repository_is_in_the_catalog_with_its_format() {
    // Arrange
    let catalog = MemoryCatalog::default();
    let ids = SequentialIdGen::default();

    // Act
    let result = block_on(ensure_hosted_repositories(&catalog, &ids, &configured()));

    // Assert
    let stored = block_on(async {
        (
            stored_id(&catalog, Format::Npm).await.is_some(),
            stored_id(&catalog, Format::Cargo).await.is_some(),
        )
    });
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(stored, (true, false));
}

#[test]
fn repeated_start_keeps_the_stored_repository_id() {
    // Arrange
    let catalog = MemoryCatalog::default();
    let ids = SequentialIdGen::default();
    block_on(ensure_hosted_repositories(&catalog, &ids, &configured())).unwrap();
    let first = block_on(stored_id(&catalog, Format::Npm));

    // Act
    let result = block_on(ensure_hosted_repositories(&catalog, &ids, &configured()));

    // Assert
    let second = block_on(stored_id(&catalog, Format::Npm));
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(second, first);
}
