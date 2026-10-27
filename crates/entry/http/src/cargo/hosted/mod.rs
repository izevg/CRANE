//! Hosted-репозитории: маршруты под `/cargo/{repo}`, где реестр хранит и публикует сам.

mod api;
mod index;

use std::sync::Arc;

use axum::Router;
use axum::extract::DefaultBodyLimit;
use axum::routing::{delete, get, put};
use crane_app::registry::Registry;
use crane_ports::registry::{BlobStore, Catalog};

use super::{CargoSettings, cargo_not_found};

/// Общее состояние обработчиков.
struct Shared<C, B> {
    registry: Arc<Registry<C, B>>,
    settings: CargoSettings,
}

/// Маршруты hosted-репозитория без слоя аутентификации: его накладывает `api_router`.
pub(super) fn hosted_router<C: Catalog + 'static, B: BlobStore + 'static>(
    registry: Arc<Registry<C, B>>,
    settings: CargoSettings,
) -> Router {
    let max_publish_bytes = settings.max_publish_bytes;
    let shared = Arc::new(Shared { registry, settings });
    Router::new()
        .route("/index/config.json", get(index::config))
        .route("/index/{*path}", get(index::sparse))
        .route(
            "/api/v1/crates/new",
            put(api::publish).layer(DefaultBodyLimit::max(max_publish_bytes)),
        )
        .route(
            "/api/v1/crates/{name}/{version}/download",
            get(api::download),
        )
        .route("/api/v1/crates/{name}/{version}/yank", delete(api::yank))
        .route("/api/v1/crates/{name}/{version}/unyank", put(api::unyank))
        .route(
            "/api/v1/crates/{name}/owners",
            get(api::owners)
                .put(api::owners_unsupported)
                .delete(api::owners_unsupported),
        )
        .fallback(cargo_not_found)
        .with_state(shared)
}
