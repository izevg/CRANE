//! Hosted-репозитории: реестр хранит и публикует сам.

mod read;
mod write;

use std::sync::Arc;

use axum::Router;
use axum::extract::rejection::BytesRejection;
use axum::extract::{DefaultBodyLimit, Extension, Path, State};
use axum::http::header;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum_extra::TypedHeader;
use axum_extra::headers::IfNoneMatch;
use bytes::Bytes;
use crane_app::registry::Registry;
use crane_domain::access::Caller;
use crane_format_npm::Route;
use crane_ports::registry::{BlobStore, Catalog};

use super::{npm_not_found, parse_route, service_reply};
use crate::error::ClientError;
use crate::formats::FormatSettings;

/// Общее состояние обработчика.
pub(super) struct Shared<C, B> {
    registry: Arc<Registry<C, B>>,
    settings: FormatSettings,
}

/// Что нужно операциям от запроса, кроме маршрута.
struct Request {
    repo: String,
    caller: Caller,
    accept: Option<String>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
    body: Bytes,
}

/// Router hosted-репозитория без слоёв аутентификации и формата: их накладывает `api_router`.
pub(crate) fn hosted_router<C: Catalog + 'static, B: BlobStore + 'static>(
    registry: Arc<Registry<C, B>>,
    settings: &FormatSettings,
) -> Router {
    let max_publish_bytes = settings.max_publish_bytes;
    let shared = Arc::new(Shared {
        registry,
        settings: settings.clone(),
    });
    Router::new()
        .route("/{*path}", any(handle::<C, B>))
        .layer(DefaultBodyLimit::max(max_publish_bytes))
        .fallback(npm_not_found)
        .with_state(shared)
}

async fn handle<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, path)): Path<(String, String)>,
    parts: Parts,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
    body: Result<Bytes, BytesRejection>,
) -> Response {
    let outcome = async {
        let route = parse_route(&parts.method, &path)?;
        if let Some(reply) = service_reply(&route, &caller) {
            return Ok(reply);
        }
        let body =
            body.map_err(|rejection| ClientError::new(rejection.status(), rejection.body_text()))?;
        let accept = parts
            .headers
            .get(header::ACCEPT)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let request = Request {
            repo,
            caller,
            accept,
            if_none_match,
            body,
        };
        dispatch(&shared, route, request).await
    }
    .await;
    outcome.unwrap_or_else(IntoResponse::into_response)
}

async fn dispatch<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    route: Route,
    request: Request,
) -> Result<Response, ClientError> {
    match route {
        Route::Packument { package } => read::packument(shared, &request, &package).await,
        Route::Tarball { package, file } => read::tarball(shared, &request, &package, &file).await,
        Route::DistTags { package } => read::dist_tags(shared, &request, &package).await,
        Route::Write { package } => write::put(shared, &request, &package).await,
        Route::DeleteTarball { package, file } => {
            write::delete_tarball(shared, &request, &package, &file).await
        }
        Route::DeletePackage { package } => write::delete_package(shared, &request, &package).await,
        Route::SetDistTag { package, tag } => {
            write::set_dist_tag(shared, &request, &package, &tag).await
        }
        Route::RemoveDistTag { package, tag } => {
            write::remove_dist_tag(shared, &request, &package, &tag).await
        }
        // Обслуживаются выше, до разбора тела.
        Route::Whoami | Route::Ping | Route::Audit => Err(ClientError::not_found()),
    }
}
