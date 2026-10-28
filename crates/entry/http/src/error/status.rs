//! Единая таблица `RegistryError` и `ProxyError` → HTTP-статус и текст для клиента.

use axum::http::StatusCode;
use crane_app::proxy::ProxyError;
use crane_app::registry::RegistryError;
use crane_domain::registry::VersionState;

use super::{ClientError, NOT_FOUND_DETAIL, UPSTREAM_DETAIL};

impl From<RegistryError> for ClientError {
    fn from(error: RegistryError) -> Self {
        describe(&error)
    }
}

/// Клиенту proxy сообщает только «upstream недоступен»: адреса, причины отказа и суммы остаются в журнале.
impl From<ProxyError> for ClientError {
    fn from(error: ProxyError) -> Self {
        match error {
            ProxyError::NotFound => Self::not_found(),
            ProxyError::Upstream(_)
            | ProxyError::DigestMismatch { .. }
            | ProxyError::TooLarge { .. } => Self::new(StatusCode::BAD_GATEWAY, UPSTREAM_DETAIL),
            ProxyError::Unreachable(_) => Self::unreachable(&error),
            ProxyError::Backend(_) => Self::internal(&error),
        }
    }
}

fn describe(error: &RegistryError) -> ClientError {
    let status = match error {
        RegistryError::RepositoryNotFound
        | RegistryError::VersionNotFound
        | RegistryError::TagNotFound
        | RegistryError::AssetNotFound => StatusCode::NOT_FOUND,
        RegistryError::NotHosted => StatusCode::METHOD_NOT_ALLOWED,
        RegistryError::VersionExists | RegistryError::AssetExists => StatusCode::CONFLICT,
        RegistryError::Unavailable {
            state: VersionState::Deleted,
        } => StatusCode::GONE,
        RegistryError::Unavailable { .. }
        | RegistryError::DeleteWindowClosed
        | RegistryError::NamespaceOwned
        | RegistryError::PublicName => StatusCode::FORBIDDEN,
        RegistryError::DigestMismatch { .. } | RegistryError::UnscopedName => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        RegistryError::Body(_) | RegistryError::Rejected { .. } => StatusCode::BAD_REQUEST,
        RegistryError::Unreachable(_) => return ClientError::unreachable(error),
        // Внутреннее имя для клиента неотличимо от отсутствующего: причину не раскрываем.
        RegistryError::Shadowed => {
            return ClientError::new(StatusCode::NOT_FOUND, NOT_FOUND_DETAIL);
        }
        RegistryError::UpstreamUnavailable => StatusCode::SERVICE_UNAVAILABLE,
        RegistryError::NamespaceConflict | RegistryError::Backend(_) => {
            return ClientError::internal(error);
        }
    };
    ClientError::new(status, error.to_string())
}

#[cfg(test)]
#[path = "tests/status.rs"]
mod tests;
