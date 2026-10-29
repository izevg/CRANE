//! Yank и unyank (PEP 592) временным маршрутом формата: форма как у admin API `yankVersion`.

use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use crane_domain::access::Caller;
use crane_ports::registry::{BlobStore, Catalog};

use super::{Shared, coordinate};
use crate::error::ClientError;

pub(super) async fn yank<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name, version)): Path<(String, String, String)>,
) -> Result<Response, ClientError> {
    set_yanked(&shared, &caller, repo, &name, &version, true).await
}

pub(super) async fn unyank<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name, version)): Path<(String, String, String)>,
) -> Result<Response, ClientError> {
    set_yanked(&shared, &caller, repo, &name, &version, false).await
}

async fn set_yanked<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    caller: &Caller,
    repo: String,
    name: &str,
    version: &str,
    yanked: bool,
) -> Result<Response, ClientError> {
    let coordinate = coordinate(repo, name, version)?;
    shared
        .registry
        .set_yanked(caller, &coordinate, yanked)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
