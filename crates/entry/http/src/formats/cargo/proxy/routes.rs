//! Обработчики proxy-репозитория crates.io.

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::IfNoneMatch;
use crane_app::proxy::Fetched;
use crane_app::registry::RegistryError;
use crane_domain::access::Caller;
use crane_domain::proxy::NegativePolicy;
use crane_domain::registry::{Checksum, Format, PackageName};
use crane_format_cargo::{archive, parse_index_path, render_config};
use crane_ports::proxy::{FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::{BlobStore, Catalog};

use super::Handle;
use crate::context::CacheFacts;
use crate::error::{ClientError, chain};
use crate::formats::cargo::INDEX_TYPE;
use crate::serve::{document_response, refuse, respond, with_facts};

const JSON: &str = "application/json";
const OCTET_STREAM: &str = "application/octet-stream";

pub(super) async fn config<U, PC, L, B, RC, RB>(
    State(shared): State<Handle<U, PC, L, B, RC, RB>>,
) -> Response {
    ([(header::CONTENT_TYPE, JSON)], render_config(&shared.base)).into_response()
}

pub(super) async fn sparse<U, PC, L, B, RC, RB>(
    State(shared): State<Handle<U, PC, L, B, RC, RB>>,
    Extension(caller): Extension<Caller>,
    Path(path): Path<String>,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
) -> Response
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
    RC: Catalog + 'static,
    RB: BlobStore + 'static,
{
    // `parse_index_path` принимает только каноничную раскладку; путь для upstream строится из имени.
    let Some(package) = parse_index_path(&path) else {
        return ClientError::not_found().into_response();
    };
    respond(shared.index(&caller, &package).await, |body| {
        document_response(body, if_none_match, INDEX_TYPE, false)
    })
}

pub(super) async fn download<U, PC, L, B, RC, RB>(
    State(shared): State<Handle<U, PC, L, B, RC, RB>>,
    Extension(caller): Extension<Caller>,
    Path((name, version)): Path<(String, String)>,
) -> Response
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
    RC: Catalog + 'static,
    RB: BlobStore + 'static,
{
    let package = match PackageName::parse(Format::Cargo, &name) {
        Ok(package) => package,
        Err(error) => return ClientError::bad_request(chain(&error)).into_response(),
    };
    let index = match shared.index(&caller, &package).await {
        Ok(index) => index,
        Err(refusal) => return refuse(refusal),
    };
    // Ревалидация индекса здесь не запускается: она ленивая и брошенной ничего не стоит,
    // а свежесть индекса поддерживают его собственные запросы.
    let Some(archive) = archive(&index.body, &version) else {
        // Версии нет в индексе: в upstream за архивом не ходим, 403 static.crates.io не нужен.
        return with_facts(
            ClientError::not_found().into_response(),
            CacheFacts::served(&index.result),
        );
    };
    let result = shared
        .engine
        .artifact(
            &shared.origins.dl,
            &archive.path,
            Some(Checksum::Sha256(archive.sha256)),
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

pub(super) async fn not_hosted() -> ClientError {
    ClientError::from(RegistryError::NotHosted)
}
