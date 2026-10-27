//! Недопустимый репозиторий назначения.

use crane_app::registry::RegistryError;
use futures_executor::block_on;

use crate::common::{Seed, caller, publish_request};

use super::HELLO;

#[test]
fn publish_to_unknown_repository_fails() {
    // Arrange
    let registry = Seed::new().registry();
    let mut request = publish_request("foo", "1.0.0", &HELLO, Vec::new());
    request.coordinate.repository = "nope".to_owned();

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::RepositoryNotFound),
        "{error:?}"
    );
}

#[test]
fn publish_to_proxy_repository_is_forbidden() {
    // Arrange
    let registry = Seed::new().with_proxy().registry();
    let mut request = publish_request("foo", "1.0.0", &HELLO, Vec::new());
    request.coordinate.repository = crate::common::PROXY.to_owned();

    // Act
    let error = block_on(registry.publish(&caller(), request)).unwrap_err();

    // Assert
    assert!(
        matches!(error, RegistryError::NotHosted),
        "в proxy публиковать нельзя: {error:?}"
    );
}
