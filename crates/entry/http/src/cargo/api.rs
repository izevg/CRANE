//! Web API cargo: публикация, скачивание, yank и владельцы.

use std::future::ready;
use std::io;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::rejection::BytesRejection;
use axum::extract::{Extension, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use crane_app::registry::{Coordinate, Publish, Upload};
use crane_domain::access::Caller;
use crane_domain::registry::{Format, PackageName, Version};
use crane_format_cargo::{asset_path, parse_publish, render_ok, render_owners, render_publish_ok};
use crane_ports::registry::{BlobStore, Catalog};
use futures_util::stream;

use super::Shared;
use crate::error::{CargoError, chain};

const JSON: &str = "application/json";

fn coordinate(repo: String, name: &str, version: &str) -> Result<Coordinate, CargoError> {
    let package =
        PackageName::parse(Format::Cargo, name).map_err(|e| CargoError::bad_request(chain(&e)))?;
    let version =
        Version::parse(Format::Cargo, version).map_err(|e| CargoError::bad_request(chain(&e)))?;
    Ok(Coordinate {
        format: Format::Cargo,
        repository: repo,
        package,
        version,
    })
}

fn json_ok(body: String) -> Response {
    ([(header::CONTENT_TYPE, JSON)], body).into_response()
}

pub(super) async fn publish<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path(repo): Path<String>,
    body: Result<Bytes, BytesRejection>,
) -> Result<Response, CargoError> {
    let body =
        body.map_err(|rejection| CargoError::new(rejection.status(), rejection.body_text()))?;
    let request = parse_publish(body).map_err(|e| CargoError::bad_request(chain(&e)))?;

    // crates.io тоже отвергает смену написания: иначе строки индекса
    // одного файла несли бы разные имена.
    let existing = shared
        .registry
        .versions(&caller, Format::Cargo, &repo, &request.name)
        .await?;
    if let Some(first) = existing.first()
        && first.package.as_written() != request.name.as_written()
    {
        return Err(CargoError::bad_request(format!(
            "крейт уже опубликован под именем `{}`",
            first.package.as_written()
        )));
    }

    let path = asset_path(&request.name, &request.version);
    let crate_file: Result<Bytes, io::Error> = Ok(request.crate_file);
    shared
        .registry
        .publish(
            &caller,
            Publish {
                coordinate: Coordinate {
                    format: Format::Cargo,
                    repository: repo,
                    package: request.name,
                    version: request.version,
                },
                file: Upload {
                    path,
                    body: Box::pin(stream::once(ready(crate_file))),
                    // Cargo хэш не присылает: cksum берётся из CAS.
                    declared_sha256: None,
                },
                metadata: request.metadata,
            },
        )
        .await?;
    Ok(json_ok(render_publish_ok()))
}

pub(super) async fn download<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name, version)): Path<(String, String, String)>,
) -> Result<Response, CargoError> {
    let coordinate = coordinate(repo, &name, &version)?;
    let path = asset_path(&coordinate.package, &coordinate.version);
    let download = shared.registry.fetch(&caller, &coordinate, &path).await?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, "application/octet-stream".to_owned()),
            (header::CONTENT_LENGTH, download.asset.size.to_string()),
        ],
        Body::from_stream(download.body),
    )
        .into_response())
}

pub(super) async fn yank<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name, version)): Path<(String, String, String)>,
) -> Result<Response, CargoError> {
    set_yanked(&shared, &caller, repo, &name, &version, true).await
}

pub(super) async fn unyank<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name, version)): Path<(String, String, String)>,
) -> Result<Response, CargoError> {
    set_yanked(&shared, &caller, repo, &name, &version, false).await
}

async fn set_yanked<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    caller: &Caller,
    repo: String,
    name: &str,
    version: &str,
    yanked: bool,
) -> Result<Response, CargoError> {
    let coordinate = coordinate(repo, name, version)?;
    shared
        .registry
        .set_yanked(caller, &coordinate, yanked)
        .await?;
    Ok(json_ok(render_ok()))
}

pub(super) async fn owners<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name)): Path<(String, String)>,
) -> Result<Response, CargoError> {
    let package =
        PackageName::parse(Format::Cargo, &name).map_err(|e| CargoError::bad_request(chain(&e)))?;
    let versions = shared
        .registry
        .versions(&caller, Format::Cargo, &repo, &package)
        .await?;
    if versions.is_empty() {
        return Err(CargoError::not_found());
    }
    Ok(json_ok(render_owners(&caller.principal.id)))
}

pub(super) async fn owners_unsupported() -> CargoError {
    CargoError::new(
        StatusCode::NOT_IMPLEMENTED,
        "управление владельцами появится в v0.4.0",
    )
}
