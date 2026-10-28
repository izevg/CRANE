//! Обработчик proxy-репозитория npm.

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::IfNoneMatch;
use crane_app::proxy::Fetched;
use crane_app::registry::RegistryError;
use crane_domain::access::Caller;
use crane_domain::proxy::NegativePolicy;
use crane_domain::registry::PackageName;
use crane_format_npm::{
    Route, TarballBase, TarballError, View, content_type, render_upstream, tarball,
};
use crane_ports::proxy::{FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::{BlobStore, Catalog};
use tracing::warn;

use super::{Handle, Shared};
use crate::context::CacheFacts;
use crate::error::{ClientError, chain};
use crate::npm::{parse_route, service_reply};
use crate::serve::{document_response, refuse, respond, with_facts};

const OCTET_STREAM: &str = "application/octet-stream";
const UPSTREAM_DETAIL: &str = "upstream недоступен";

pub(super) async fn handle<U, PC, L, B, RC, RB>(
    State(shared): State<Handle<U, PC, L, B, RC, RB>>,
    Extension(caller): Extension<Caller>,
    Path(path): Path<String>,
    method: Method,
    headers: HeaderMap,
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
    let route = match parse_route(&method, &path) {
        Ok(route) => route,
        Err(error) => return error.into_response(),
    };
    if let Some(reply) = service_reply(&route, &caller) {
        return reply;
    }
    match route {
        Route::Packument { package } => {
            let accept = headers.get(header::ACCEPT).and_then(|v| v.to_str().ok());
            packument(
                &shared,
                &caller,
                &package,
                View::from_accept(accept),
                if_none_match,
            )
            .await
        }
        Route::Tarball { package, file } => artifact(&shared, &caller, &package, &file).await,
        Route::Write { .. }
        | Route::DeleteTarball { .. }
        | Route::DeletePackage { .. }
        | Route::SetDistTag { .. }
        | Route::RemoveDistTag { .. } => {
            ClientError::from(RegistryError::NotHosted).into_response()
        }
        // Список тегов proxy не ведёт: его знает только upstream. Служебные маршруты обслужены выше.
        Route::DistTags { .. } | Route::Whoami | Route::Ping | Route::Audit => {
            ClientError::not_found().into_response()
        }
    }
}

async fn packument<U, PC, L, B, RC, RB>(
    shared: &Shared<U, PC, L, B, RC, RB>,
    caller: &Caller,
    package: &PackageName,
    view: View,
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
    respond(shared.packument(caller, package).await, |raw| {
        let body =
            render_upstream(&raw, package, view, TarballBase(&shared.base)).map_err(|error| {
                // Upstream прислал мусор: клиенту — 502, подробности только журналу.
                warn!(error = %chain(&error), "документ upstream не разобран");
                ClientError::new(StatusCode::BAD_GATEWAY, UPSTREAM_DETAIL)
            })?;
        document_response(body.into(), if_none_match, content_type(view), true)
    })
}

async fn artifact<U, PC, L, B, RC, RB>(
    shared: &Shared<U, PC, L, B, RC, RB>,
    caller: &Caller,
    package: &PackageName,
    file: &str,
) -> Response
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
    RC: Catalog + 'static,
    RB: BlobStore + 'static,
{
    let document = match shared.packument(caller, package).await {
        Ok(document) => document,
        Err(refusal) => return refuse(refusal),
    };
    // Ревалидация документа здесь не запускается: она ленивая и брошенной ничего не стоит,
    // а свежесть документа поддерживают его собственные запросы.
    let found = match tarball(&document.body, package, file) {
        Ok(found) => found,
        Err(TarballError::NotListed) => {
            return with_facts(
                ClientError::not_found().into_response(),
                CacheFacts::served(&document.result),
            );
        }
        // Без sha512 проверить архив нечем: в upstream за ним не ходим.
        Err(TarballError::WeakIntegrity) => {
            return with_facts(
                ClientError::new(StatusCode::BAD_GATEWAY, UPSTREAM_DETAIL).into_response(),
                CacheFacts::weak_integrity(&document.result),
            );
        }
    };
    let result = shared
        .engine
        .artifact(
            &shared.origin,
            &found.path,
            Some(found.expected),
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
