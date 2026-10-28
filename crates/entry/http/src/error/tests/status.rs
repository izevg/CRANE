#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use std::io;
use std::sync::Arc;

use crane_app::proxy::UpstreamFailure;
use crane_domain::proxy::EgressRefusal;
use crane_domain::registry::{Checksum, Sha256};

use super::super::{INTERNAL_DETAIL, UNREACHABLE_DETAIL};
use super::*;

#[test]
fn registry_errors_map_to_http_statuses() {
    // Arrange
    let digest = Checksum::Sha256(Sha256::from([1; 32]));
    let cases = [
        (RegistryError::RepositoryNotFound, StatusCode::NOT_FOUND),
        (RegistryError::NotHosted, StatusCode::METHOD_NOT_ALLOWED),
        (RegistryError::VersionNotFound, StatusCode::NOT_FOUND),
        (RegistryError::AssetNotFound, StatusCode::NOT_FOUND),
        (RegistryError::VersionExists, StatusCode::CONFLICT),
        (
            RegistryError::Unavailable {
                state: VersionState::Deleted,
            },
            StatusCode::GONE,
        ),
        (
            RegistryError::Unavailable {
                state: VersionState::Quarantined,
            },
            StatusCode::FORBIDDEN,
        ),
        (
            RegistryError::Unavailable {
                state: VersionState::Blocked,
            },
            StatusCode::FORBIDDEN,
        ),
        (
            RegistryError::DigestMismatch {
                declared: Box::new(digest),
                actual: Box::new(digest),
            },
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
        (RegistryError::DeleteWindowClosed, StatusCode::FORBIDDEN),
        (RegistryError::TagNotFound, StatusCode::NOT_FOUND),
        (
            RegistryError::Body(io::Error::other("обрыв")),
            StatusCode::BAD_REQUEST,
        ),
        (
            RegistryError::Unreachable("сбой".into()),
            StatusCode::SERVICE_UNAVAILABLE,
        ),
        (
            RegistryError::Backend("сбой".into()),
            StatusCode::INTERNAL_SERVER_ERROR,
        ),
    ];

    // Act
    let actual = cases.each_ref().map(|(error, _)| describe(error).status);

    // Assert
    assert_eq!(actual, cases.map(|(_, expected)| expected));
}

#[test]
fn proxy_errors_map_to_status_and_hide_upstream_details() {
    // Arrange
    let sum = Checksum::Sha256(Sha256::from([1; 32]));
    let cases = [
        ProxyError::NotFound,
        ProxyError::Upstream(UpstreamFailure::Unavailable),
        ProxyError::Upstream(UpstreamFailure::Refused(EgressRefusal::HostNotAllowed(
            "evil.example".to_owned(),
        ))),
        ProxyError::DigestMismatch {
            expected: Box::new(sum),
            actual: Box::new(sum),
        },
        ProxyError::TooLarge { limit: 1 },
        ProxyError::Unreachable(Arc::new(io::Error::other("postgres://u:SECRET@h/db"))),
        ProxyError::Backend(Arc::new(io::Error::other("postgres://u:SECRET@h/db"))),
    ];

    // Act
    let actual = cases.map(|error| {
        let error = ClientError::from(error);
        (error.status, error.detail)
    });

    // Assert
    let bad_gateway = (StatusCode::BAD_GATEWAY, UPSTREAM_DETAIL.to_owned());
    assert_eq!(
        actual,
        [
            (StatusCode::NOT_FOUND, "не найдено".to_owned()),
            bad_gateway.clone(),
            bad_gateway.clone(),
            bad_gateway.clone(),
            bad_gateway,
            (
                StatusCode::SERVICE_UNAVAILABLE,
                UNREACHABLE_DETAIL.to_owned()
            ),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                INTERNAL_DETAIL.to_owned()
            ),
        ]
    );
}

#[test]
fn backend_detail_hides_the_source_message() {
    // Arrange
    let error = RegistryError::Backend("postgres://u:SECRET@h/db".into());

    // Act
    let detail = describe(&error).detail;

    // Assert
    assert_eq!(detail, INTERNAL_DETAIL);
}

#[test]
fn unreachable_detail_hides_the_source_message() {
    // Arrange
    let error = RegistryError::Unreachable("postgres://u:SECRET@h/db".into());

    // Act
    let detail = describe(&error).detail;

    // Assert
    assert_eq!(detail, UNREACHABLE_DETAIL);
}

#[test]
fn client_facing_detail_keeps_the_error_text() {
    // Arrange
    let error = RegistryError::VersionExists;

    // Act
    let detail = describe(&error).detail;

    // Assert
    assert_eq!(detail, "версия уже существует");
}
