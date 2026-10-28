//! Защита имён при первой загрузке: имя из публичного реестра без резервации не захватывается.

use std::sync::Arc;

use axum::http::StatusCode;
use crane_domain::registry::Format;
use crane_ports::proxy::Presence;

use crate::common::Api;
use crate::common::names::FixedPresence;
use crate::fixtures::{Dist, simple_uri};

#[test]
fn first_upload_of_a_name_known_to_the_public_registry_is_forbidden() {
    // Arrange
    let api = Api::with_upstream(Arc::new(FixedPresence(Presence::Public)));

    // Act
    let reply = api.pypi_upload(&Dist::wheel("acme-core", "1.0.0"));

    // Assert
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(
        api.pypi_get(&simple_uri("acme-core"), None).status,
        StatusCode::NOT_FOUND
    );
}

#[test]
fn first_upload_while_the_public_registry_is_unreachable_is_unavailable() {
    // Arrange
    let api = Api::with_upstream(Arc::new(FixedPresence(Presence::Unknown)));

    // Act
    let reply = api.pypi_upload(&Dist::wheel("acme-core", "1.0.0"));

    // Assert
    assert_eq!(reply.status, StatusCode::SERVICE_UNAVAILABLE);
}

#[test]
fn name_covered_by_a_reserved_namespace_is_accepted_even_if_it_is_public() {
    // Arrange
    let api = Api::with_upstream(Arc::new(FixedPresence(Presence::Public)));
    api.reserve(Format::PyPi, "acme-*");

    // Act
    let reply = api.pypi_upload(&Dist::wheel("acme-core", "1.0.0"));

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}
