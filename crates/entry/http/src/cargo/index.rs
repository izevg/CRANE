//! `config.json` и sparse-индекс с условными запросами.

use std::sync::Arc;

use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::{ETag, IfNoneMatch};
use crane_domain::registry::{Format, Sha256 as Digest};
use crane_format_cargo::{parse_index_path, render_config, render_index};
use crane_ports::registry::{BlobStore, Catalog};
use sha2::{Digest as _, Sha256};

use super::Shared;
use crate::error::CargoError;

const JSON: &str = "application/json";
const INDEX_TYPE: &str = "text/plain; charset=utf-8";

pub(super) async fn config<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Path(repo): Path<String>,
) -> Response {
    // Существование репозитория здесь не проверяется: use case «репозиторий есть?» нет,
    // неизвестный репозиторий проявится 404 на остальных маршрутах.
    let base = format!("{}/cargo/{repo}", shared.settings.public_url);
    ([(header::CONTENT_TYPE, JSON)], render_config(&base)).into_response()
}

pub(super) async fn sparse<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Path((repo, path)): Path<(String, String)>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> Result<Response, CargoError> {
    let name = parse_index_path(&path).ok_or_else(CargoError::not_found)?;
    let versions = shared
        .registry
        .versions(Format::Cargo, &repo, &name)
        .await?;
    // Пустой индекс — это «крейта нет», а не крейт без версий.
    if versions.is_empty() {
        return Err(CargoError::not_found());
    }
    let body = render_index(&versions).map_err(|error| CargoError::internal(&error))?;
    let etag = etag_of(body.as_bytes())?;

    let unchanged =
        if_none_match.is_some_and(|TypedHeader(condition)| !condition.precondition_passes(&etag));
    if unchanged {
        return Ok((StatusCode::NOT_MODIFIED, TypedHeader(etag)).into_response());
    }
    Ok((
        TypedHeader(etag),
        [(header::CONTENT_TYPE, HeaderValue::from_static(INDEX_TYPE))],
        body,
    )
        .into_response())
}

/// `ETag` считается по тем самым байтам, что уходят клиенту.
fn etag_of(body: &[u8]) -> Result<ETag, CargoError> {
    let digest = Digest::from(<[u8; 32]>::from(Sha256::digest(body)));
    format!("\"{digest}\"")
        .parse()
        .map_err(|error| CargoError::internal(&error))
}
