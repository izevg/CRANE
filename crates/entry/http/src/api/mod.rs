//! Корневой router API: все форматы под токеном, всё остальное — problem+json 404.

use std::sync::Arc;

use axum::Router;
use axum::middleware::from_fn_with_state;
use crane_app::registry::Registry;
use crane_domain::registry::Format;
use crane_ports::audit::AuditSink;
use crane_ports::registry::{BlobStore, Catalog};

use crate::auth::{Auth, StaticToken, require_token};
use crate::context::{TrustedProxies, request_context};
use crate::error::{mark_format, not_found};
use crate::formats::{self, FormatSettings};

/// Корневой router API: форматы под токеном, всё остальное — problem+json 404.
///
/// `proxy_repos` — готовые routers proxy-репозиториев; каждый монтируется как `/<формат>/<имя>`.
/// Имя должно быть допустимым сегментом пути axum (без `{`, `}`, `/`).
pub fn api_router<C: Catalog + 'static, B: BlobStore + 'static>(
    registry: &Arc<Registry<C, B>>,
    settings: &FormatSettings,
    token: StaticToken,
    proxies: TrustedProxies,
    audit: Arc<dyn AuditSink>,
    proxy_repos: Vec<(Format, String, Router)>,
) -> Router {
    let token = Arc::new(token);
    // `layer`, а не `route_layer`: 401 должен закрывать и неизвестные пути, иначе без токена можно
    // перебирать маршруты. Формат снаружи auth: отказы аутентификации тоже в форме своего формата.
    let guard = move |format: Format, router: Router| {
        let auth = Arc::new(Auth {
            token: Arc::clone(&token),
            audit: Arc::clone(&audit),
            format,
        });
        router
            .layer(from_fn_with_state(auth, require_token))
            .layer(from_fn_with_state(format, mark_format))
    };
    let mut root = Router::new();
    for format in Format::ALL {
        if let Some(hosted) = formats::hosted_router(format, registry, settings) {
            root = root.nest(
                &format!("/{}/{{repo}}", format.as_str()),
                guard(format, hosted),
            );
        }
    }
    // Статический сегмент побеждает `{repo}` в маршрутизаторе axum, поэтому proxy-имя
    // не перехватывается hosted-маршрутами (проверено в tests/cargo_proxy.rs и tests/npm_proxy.rs).
    for (format, name, router) in proxy_repos {
        root = root.nest(
            &format!("/{}/{name}", format.as_str()),
            guard(format, router),
        );
    }
    // Самый внешний слой: контекст запроса нужен и auth, и fallback, и рендеру ошибок.
    root.fallback(not_found)
        .layer(from_fn_with_state(Arc::new(proxies), request_context))
}
