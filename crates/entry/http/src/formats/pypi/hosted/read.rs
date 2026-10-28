//! Чтение: страница проекта (PEP 503 и PEP 691) и файлы.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::IfNoneMatch;
use bytes::Bytes;
use crane_domain::access::Caller;
use crane_domain::registry::{Ecosystem, Format, PackageName};
use crane_format_pypi::{PYPI, View, content_type, hosted, render};
use crane_ports::registry::{BlobStore, Catalog};

use super::{Shared, coordinate};
use crate::error::ClientError;
use crate::serve::document_response;

const OCTET_STREAM: &str = "application/octet-stream";

/// Имя, которое не разбирается, не может быть проектом: клиенту это «нет такого проекта».
/// pip при 400 прервал бы разрешение зависимостей, при 404 идёт к следующему индексу.
fn project_name(raw: &str) -> Result<PackageName, ClientError> {
    PYPI.parse_name(raw).map_err(|_| ClientError::not_found())
}

/// Относительный `Location`: он не зависит от внешнего адреса и от того, как имя репозитория
/// попало в путь.
fn moved_to(location: &str) -> Result<Response, ClientError> {
    let location = HeaderValue::from_str(location).map_err(|_| ClientError::not_found())?;
    Ok((
        StatusCode::MOVED_PERMANENTLY,
        [(header::LOCATION, location)],
    )
        .into_response())
}

pub(super) async fn project<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name)): Path<(String, String)>,
    headers: HeaderMap,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> Result<Response, ClientError> {
    let package = project_name(&name)?;
    // PEP 503: клиент обязан запрашивать нормализованное имя, а сервер вправе его подсказать.
    if name != package.normalized() {
        return moved_to(&format!("../{}/", package.normalized()));
    }
    let view = View::from_accept(
        headers
            .get(header::ACCEPT)
            .and_then(|value| value.to_str().ok()),
    );
    let versions = shared
        .registry
        .versions(&caller, Format::PyPi, &repo, &package)
        .await?;
    // Проект без видимых версий для клиента не существует.
    if versions.is_empty() {
        return Err(ClientError::not_found());
    }
    let base = shared.settings.repo_url(Format::PyPi, &repo);
    let body = render(&hosted(&package, &versions, &base), view);
    document_response(Bytes::from(body), if_none_match, content_type(view), true)
}

/// PEP 503: адрес без завершающего `/` перенаправляется на адрес со слэшем.
pub(super) async fn project_without_slash(
    Path((_repo, name)): Path<(String, String)>,
) -> Result<Response, ClientError> {
    let package = project_name(&name)?;
    moved_to(&format!("{}/", package.normalized()))
}

pub(super) async fn file<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name, version, filename)): Path<(String, String, String, String)>,
) -> Result<Response, ClientError> {
    let coordinate = coordinate(repo, &name, &version).map_err(|_| ClientError::not_found())?;
    let download = shared
        .registry
        .fetch(&caller, &coordinate, &filename)
        .await?;
    Ok((
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, OCTET_STREAM.to_owned()),
            (header::CONTENT_LENGTH, download.asset.size.to_string()),
        ],
        Body::from_stream(download.body),
    )
        .into_response())
}
