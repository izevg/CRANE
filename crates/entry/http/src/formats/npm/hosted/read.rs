//! Чтение: packument, tarball, dist-tags.

use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use crane_domain::registry::{Coordinate, Format, PackageName};
use crane_format_npm::{
    TarballBase, View, content_type, effective_tags, render_hosted, render_tags, tarball_version,
};
use crane_ports::registry::{BlobStore, Catalog};

use super::{Request, Shared};
use crate::error::ClientError;
use crate::formats::npm::json_ok;
use crate::serve::document_response;

const OCTET_STREAM: &str = "application/octet-stream";

pub(super) fn coordinate(
    request: &Request,
    package: &PackageName,
    version: crane_domain::registry::Version,
) -> Coordinate {
    Coordinate {
        format: Format::Npm,
        repository: request.repo.clone(),
        package: package.clone(),
        version,
    }
}

pub(super) async fn packument<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    request: &Request,
    package: &PackageName,
) -> Result<Response, ClientError> {
    let view = View::from_accept(request.accept.as_deref());
    let stored = shared
        .registry
        .package(&request.caller, Format::Npm, &request.repo, package)
        .await?;
    // Пустой документ — это «пакета нет», а не пакет без версий.
    if stored.versions.is_empty() {
        return Err(ClientError::not_found());
    }
    let base = shared.settings.repo_url(Format::Npm, &request.repo);
    let body = render_hosted(
        package,
        &stored.versions,
        &stored.tags,
        view,
        TarballBase(&base),
    )
    .map_err(|error| ClientError::internal(&error))?;
    document_response(
        Bytes::from(body),
        request.if_none_match.clone(),
        content_type(view),
        true,
    )
}

pub(super) async fn tarball<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    request: &Request,
    package: &PackageName,
    file: &str,
) -> Result<Response, ClientError> {
    let version = tarball_version(package, file).ok_or_else(ClientError::not_found)?;
    let coordinate = coordinate(request, package, version);
    let download = shared
        .registry
        .fetch(&request.caller, &coordinate, file)
        .await?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, OCTET_STREAM.to_owned()),
            (
                header::CONTENT_LENGTH,
                download.asset.content.size.to_string(),
            ),
        ],
        Body::from_stream(download.body),
    )
        .into_response())
}

/// Теги с тем же выводом `latest`, что в packument: правило одно, в формате.
pub(super) async fn dist_tags<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    request: &Request,
    package: &PackageName,
) -> Result<Response, ClientError> {
    let stored = shared
        .registry
        .package(&request.caller, Format::Npm, &request.repo, package)
        .await?;
    if stored.versions.is_empty() {
        return Err(ClientError::not_found());
    }
    Ok(json_ok(render_tags(effective_tags(
        &stored.versions,
        &stored.tags,
    ))))
}
