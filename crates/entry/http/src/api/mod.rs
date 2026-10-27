//! Корневой router API: все форматы под токеном, всё остальное — problem+json 404.

use std::sync::Arc;

use axum::Router;
use axum::middleware::from_fn_with_state;
use crane_app::registry::Registry;
use crane_ports::audit::AuditSink;
use crane_ports::registry::{BlobStore, Catalog};

use crate::auth::{Auth, StaticToken, require_token};
use crate::cargo::{self, CARGO_PREFIX, CargoSettings};
use crate::context::{TrustedProxies, request_context};
use crate::error::{Dialect, mark_dialect, not_found};
use crate::npm::{self, NPM_PREFIX, NpmSettings};

/// Готовые routers proxy-репозиториев по форматам; каждый монтируется как `/<формат>/<имя>`.
/// Имя должно быть допустимым сегментом пути axum (без `{`, `}`, `/`).
#[derive(Debug, Default)]
pub struct ProxyRouters {
    pub cargo: Vec<(String, Router)>,
    pub npm: Vec<(String, Router)>,
}

/// Настройки маршрутов форматов.
#[derive(Debug, Clone)]
pub struct FormatSettings {
    pub cargo: CargoSettings,
    pub npm: NpmSettings,
}

/// Корневой router API: форматы под токеном, всё остальное — problem+json 404.
pub fn api_router<C: Catalog + 'static, B: BlobStore + 'static>(
    registry: Arc<Registry<C, B>>,
    settings: FormatSettings,
    token: StaticToken,
    proxies: TrustedProxies,
    audit: Arc<dyn AuditSink>,
    proxy_repos: ProxyRouters,
) -> Router {
    let token = Arc::new(token);
    // `layer`, а не `route_layer`: 401 должен закрывать и неизвестные пути, иначе без токена можно
    // перебирать маршруты. Диалект снаружи auth: отказы аутентификации тоже в форме своего формата.
    let auth = |dialect: Dialect, token: Arc<StaticToken>, audit: Arc<dyn AuditSink>| {
        let auth = Arc::new(Auth {
            token,
            audit,
            dialect,
        });
        move |router: Router| {
            router
                .layer(from_fn_with_state(Arc::clone(&auth), require_token))
                .layer(from_fn_with_state(dialect, mark_dialect))
        }
    };
    let cargo_guard = auth(Dialect::Cargo, Arc::clone(&token), Arc::clone(&audit));
    let npm_guard = auth(Dialect::Npm, token, Arc::clone(&audit));
    let mut root = Router::new()
        .nest(
            &format!("{CARGO_PREFIX}/{{repo}}"),
            cargo_guard(cargo::hosted_router(
                Arc::clone(&registry),
                settings.cargo,
                audit,
            )),
        )
        .nest(
            &format!("{NPM_PREFIX}/{{repo}}"),
            npm_guard(npm::hosted_router(registry, settings.npm)),
        );
    // Статический сегмент побеждает `{repo}` в маршрутизаторе axum, поэтому proxy-имя
    // не перехватывается hosted-маршрутами (проверено в tests/cargo_proxy.rs и tests/npm_proxy.rs).
    for (name, router) in proxy_repos.cargo {
        root = root.nest(&format!("{CARGO_PREFIX}/{name}"), cargo_guard(router));
    }
    for (name, router) in proxy_repos.npm {
        root = root.nest(&format!("{NPM_PREFIX}/{name}"), npm_guard(router));
    }
    // Самый внешний слой: контекст запроса нужен и auth, и fallback, и рендеру ошибок.
    root.fallback(not_found)
        .layer(from_fn_with_state(Arc::new(proxies), request_context))
}
