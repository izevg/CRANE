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
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::{Format, PackageName, Version};
use crane_format_cargo::{
    MetadataError, asset_path, parse_publish, render_ok, render_owners, render_publish_ok,
};
use crane_ports::registry::{BlobStore, Catalog};
use futures_util::stream;

use super::{Shared, repo_base};
use crate::error::{ClientError, chain};

const JSON: &str = "application/json";

fn coordinate(repo: String, name: &str, version: &str) -> Result<Coordinate, ClientError> {
    let package =
        PackageName::parse(Format::Cargo, name).map_err(|e| ClientError::bad_request(chain(&e)))?;
    let version =
        Version::parse(Format::Cargo, version).map_err(|e| ClientError::bad_request(chain(&e)))?;
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
) -> Result<Response, ClientError> {
    let body =
        body.map_err(|rejection| ClientError::new(rejection.status(), rejection.body_text()))?;
    let request = parse_publish(body).map_err(|e| ClientError::bad_request(chain(&e)))?;
    let coordinate = Coordinate {
        format: Format::Cargo,
        repository: repo,
        package: request.name.clone(),
        version: request.version.clone(),
    };
    // Адрес индекса строится так же, как база в `config.json`: именно его cargo подставляет
    // в `registry` зависимостей этого репозитория.
    let own_index = format!(
        "sparse+{}/index/",
        repo_base(&shared.settings, &coordinate.repository)
    );
    let metadata = match request.index_metadata(&own_index) {
        Ok(metadata) => metadata,
        Err(MetadataError::Foreign(foreign)) => {
            // Попытка увести сборки потребителей мимо CRANE: это событие ИБ, а не просто 400.
            // Значения недоверенные, `{:?}` экранирует управляющие символы в журнале.
            shared.audit.record(AuditEvent {
                actor: Some(caller.principal.clone()),
                request: caller.request.clone(),
                action: AuditAction::Publish(coordinate),
                outcome: Outcome::Failure {
                    reason: format!(
                        "сторонний реестр: зависимость {:?}, registry {:?}",
                        foreign.dependency, foreign.registry
                    ),
                },
            });
            return Err(ClientError::bad_request(chain(&foreign)));
        }
        Err(error @ MetadataError::Serialize(_)) => return Err(ClientError::internal(&error)),
    };

    // crates.io тоже отвергает смену написания, и мы отказываем явной ошибкой. Каталог при этом
    // сам держит инвариант: первое записанное написание побеждает даже в гонке (`name_display`
    // не перезаписывается), поэтому проигравший гонку получит версию под первым написанием,
    // а не строки индекса с разными именами. Проверка нужна только ради понятного отказа.
    let existing = shared
        .registry
        .versions(
            &caller,
            Format::Cargo,
            &coordinate.repository,
            &coordinate.package,
        )
        .await?;
    if let Some(first) = existing.first()
        && first.package.as_written() != coordinate.package.as_written()
    {
        return Err(ClientError::bad_request(format!(
            "крейт уже опубликован под именем `{}`",
            first.package.as_written()
        )));
    }

    let path = asset_path(&coordinate.package, &coordinate.version);
    let crate_file: Result<Bytes, io::Error> = Ok(request.crate_file);
    shared
        .registry
        .publish(
            &caller,
            Publish {
                coordinate,
                file: Upload {
                    path,
                    body: Box::pin(stream::once(ready(crate_file))),
                    // Cargo хэш не присылает: cksum берётся из CAS.
                    declared: Vec::new(),
                },
                metadata,
                tags: Vec::new(),
            },
        )
        .await?;
    Ok(json_ok(render_publish_ok()))
}

pub(super) async fn download<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name, version)): Path<(String, String, String)>,
) -> Result<Response, ClientError> {
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
    Ok(json_ok(render_ok()))
}

pub(super) async fn owners<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path((repo, name)): Path<(String, String)>,
) -> Result<Response, ClientError> {
    let package = PackageName::parse(Format::Cargo, &name)
        .map_err(|e| ClientError::bad_request(chain(&e)))?;
    let versions = shared
        .registry
        .versions(&caller, Format::Cargo, &repo, &package)
        .await?;
    if versions.is_empty() {
        return Err(ClientError::not_found());
    }
    Ok(json_ok(render_owners(&caller.principal.id)))
}

pub(super) async fn owners_unsupported() -> ClientError {
    ClientError::new(
        StatusCode::NOT_IMPLEMENTED,
        "управление владельцами появится в v0.4.0",
    )
}
