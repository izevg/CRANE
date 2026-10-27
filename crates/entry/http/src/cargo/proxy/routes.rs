//! Обработчики proxy-репозитория crates.io.

use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::IfNoneMatch;
use crane_app::proxy::Fetched;
use crane_app::registry::RegistryError;
use crane_domain::proxy::{NegativePolicy, UpstreamPath};
use crane_domain::registry::{Format, PackageName};
use crane_format_cargo::{INDEX, archive, index_path, parse_index_path, render_config};
use crane_ports::proxy::{FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::BlobStore;

use super::Shared;
use super::respond::{respond, with_facts};
use crate::cargo::index_response;
use crate::context::CacheFacts;
use crate::error::{CargoError, chain};

const JSON: &str = "application/json";
const OCTET_STREAM: &str = "application/octet-stream";

pub(super) async fn config<U, PC, L, B>(
    State(shared): State<Arc<Shared<U, PC, L, B>>>,
) -> Response {
    ([(header::CONTENT_TYPE, JSON)], render_config(&shared.base)).into_response()
}

pub(super) async fn sparse<U, PC, L, B>(
    State(shared): State<Arc<Shared<U, PC, L, B>>>,
    Path(path): Path<String>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> Response
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
{
    // `parse_index_path` принимает только каноничную раскладку, поэтому в upstream уходит
    // ровно тот путь, который сам cargo построил бы для этого крейта.
    if parse_index_path(&path).is_none() {
        return CargoError::not_found().into_response();
    }
    let Ok(upstream_path) = UpstreamPath::parse(&path) else {
        return CargoError::not_found().into_response();
    };
    let result = shared
        .engine
        .metadata(&shared.origins.index, &upstream_path, &INDEX)
        .await;
    respond(result, |body| index_response(body, if_none_match))
}

pub(super) async fn download<U, PC, L, B>(
    State(shared): State<Arc<Shared<U, PC, L, B>>>,
    Path((name, version)): Path<(String, String)>,
) -> Response
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
{
    let package = match PackageName::parse(Format::Cargo, &name) {
        Ok(package) => package,
        Err(error) => return CargoError::bad_request(chain(&error)).into_response(),
    };
    let Ok(index) = UpstreamPath::parse(&index_path(&package)) else {
        return CargoError::not_found().into_response();
    };
    let index = match shared
        .engine
        .metadata(&shared.origins.index, &index, &INDEX)
        .await
    {
        Ok(index) => index,
        Err(error) => return respond::<()>(Err(error), |()| Ok(().into_response())),
    };
    // Ревалидация индекса здесь не запускается: она ленивая и брошенной ничего не стоит,
    // а свежесть индекса поддерживают его собственные запросы.
    let Some(archive) = archive(&index.body, &version) else {
        // Версии нет в индексе: в upstream за архивом не ходим, 403 static.crates.io не нужен.
        return with_facts(
            CargoError::not_found().into_response(),
            CacheFacts::served(&index.result),
        );
    };
    let result = shared
        .engine
        .artifact(
            &shared.origins.dl,
            &archive.path,
            Some(archive.sha256),
            &NegativePolicy::ARTIFACT,
        )
        .await;
    respond(result, |fetched| Ok(archive_response(fetched)))
}

fn archive_response(fetched: Fetched) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, OCTET_STREAM.to_owned()),
            (header::CONTENT_LENGTH, fetched.digests.size.to_string()),
        ],
        Body::from_stream(fetched.body),
    )
        .into_response()
}

pub(super) async fn not_hosted() -> CargoError {
    CargoError::from(RegistryError::NotHosted)
}
