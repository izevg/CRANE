//! `config.json` и sparse-индекс с условными запросами.

use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::IfNoneMatch;
use bytes::Bytes;
use crane_domain::access::Caller;
use crane_domain::registry::Format;
use crane_format_cargo::{parse_index_path, render_config, render_index};
use crane_ports::registry::{BlobStore, Catalog};

use super::Shared;
use crate::cargo::{CARGO_PREFIX, index_response};
use crate::error::CargoError;

const JSON: &str = "application/json";

pub(super) async fn config<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path(repo): Path<String>,
) -> Result<Response, CargoError> {
    // Клиент по 404 на config.json понимает, что адрес реестра неверен, а не получает
    // рабочий конфиг для репозитория, которого нет.
    shared
        .registry
        .repository(&caller, Format::Cargo, &repo)
        .await?;
    let base = format!("{}{CARGO_PREFIX}/{repo}", shared.settings.public_url);
    Ok(([(header::CONTENT_TYPE, JSON)], render_config(&base)).into_response())
}

pub(super) async fn sparse<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, path)): Path<(String, String)>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> Result<Response, CargoError> {
    let name = parse_index_path(&path).ok_or_else(CargoError::not_found)?;
    let versions = shared
        .registry
        .versions(&caller, Format::Cargo, &repo, &name)
        .await?;
    // Пустой индекс — это «крейта нет», а не крейт без версий.
    if versions.is_empty() {
        return Err(CargoError::not_found());
    }
    let body = render_index(&versions).map_err(|error| CargoError::internal(&error))?;
    index_response(Bytes::from(body), if_none_match)
}
