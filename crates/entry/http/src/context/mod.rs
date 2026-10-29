//! Внешний слой запроса: контекст, `X-Request-Id`, рендер ошибок и строка access-журнала.
//!
//! Пир берётся из `ConnectInfo`; без него (тесты через `oneshot`) он считается неизвестным
//! и недоверенным. Заголовкам клиента верят только от доверенных прокси.

mod proxies;
mod render;
mod trace;

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Instant;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use crane_app::proxy::{CacheResult, ProxyError, UpstreamFailure};
use crane_domain::access::{RequestContext, RequestId};
use tracing::Instrument;
use uuid::Uuid;

pub use proxies::{ProxyParseError, TrustedProxies};

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");
const FORWARDED_FOR: &str = "x-forwarded-for";
const TRACEPARENT: &str = "traceparent";
/// User-Agent попадает в журналы, поэтому длина ограничена.
const USER_AGENT_MAX_CHARS: usize = 256;

/// Принципал успешной аутентификации: слой auth кладёт его в расширения ответа,
/// чтобы access-журнал знал «кто» даже для чтений.
#[derive(Debug, Clone)]
pub(crate) struct Actor(pub(crate) String);

/// Откуда пришёл ответ proxy и почему upstream отказал: handler кладёт это в расширения ответа,
/// `request_context` переносит в строку access-журнала (поля `cache`, `upstream_failure`, `upstream_reason`).
/// Отказ egress пишется только сюда: access-журнал — журнал ИБ (14.9), второго события рядом не нужно.
#[derive(Debug, Clone, Default)]
pub(crate) struct CacheFacts {
    cache: Option<&'static str>,
    failure: Option<&'static str>,
    reason: Option<String>,
}

impl CacheFacts {
    /// Ответ из движка без ошибки; `StaleOnError` несёт и причину сбоя upstream.
    pub(crate) fn served(result: &CacheResult) -> Self {
        let (cache, failure) = match result {
            CacheResult::Hit => ("hit", None),
            CacheResult::Miss => ("miss", None),
            CacheResult::Stale => ("stale", None),
            CacheResult::StaleOnError(failure) => ("stale-on-error", Some(failure)),
        };
        let (failure, reason) = failure.map_or((None, None), upstream_failure);
        Self {
            cache: Some(cache),
            failure,
            reason,
        }
    }

    /// Документ отдан, но tarball не проверить (нет sha512): факты документа плюс причина отказа.
    pub(crate) fn weak_integrity(result: &CacheResult) -> Self {
        Self {
            failure: Some("weak-integrity"),
            ..Self::served(result)
        }
    }

    /// Ошибка движка; `NotFound`, недоступность хранилища и сбой бэкенда не про upstream, поэтому фактов нет.
    pub(crate) fn failed(error: &ProxyError) -> Self {
        let (failure, reason) = match error {
            ProxyError::Upstream(failure) => upstream_failure(failure),
            // Ожидаемый и полученный sha256 — событие цепочки поставки, их место в журнале.
            ProxyError::DigestMismatch { .. } => (Some("digest-mismatch"), Some(error.to_string())),
            ProxyError::TooLarge { .. } => (Some("too-large"), None),
            ProxyError::NotFound | ProxyError::Unreachable(_) | ProxyError::Backend(_) => {
                (None, None)
            }
        };
        Self {
            cache: None,
            failure,
            reason,
        }
    }
}

fn upstream_failure(failure: &UpstreamFailure) -> (Option<&'static str>, Option<String>) {
    match failure {
        UpstreamFailure::Refused(refusal) => (Some("refused"), Some(refusal.to_string())),
        UpstreamFailure::Unavailable => (Some("unavailable"), None),
        UpstreamFailure::Throttled => (Some("throttled"), None),
    }
}

fn text<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

/// Цель `tracing` лога доступа: единственный след чтений с `actor`, часть журнала ИБ
/// (раздел 14.9). Сборка сервера держит её включённой при любом `RUST_LOG`.
pub const ACCESS_TARGET: &str = "crane::access";

fn user_agent(headers: &HeaderMap) -> Option<String> {
    text(headers, "user-agent").map(|ua| ua.chars().take(USER_AGENT_MAX_CHARS).collect())
}

fn context(peer: Option<IpAddr>, headers: &HeaderMap, proxies: &TrustedProxies) -> RequestContext {
    let peer_trusted = peer.is_some_and(|ip| proxies.contains(ip));
    // Чужой X-Request-Id принимается только от своего прокси: иначе клиент подсунул бы
    // в журнал идентификатор чужого запроса и испортил корреляцию.
    let forwarded_id = peer_trusted
        .then(|| text(headers, REQUEST_ID.as_str()).and_then(RequestId::parse))
        .flatten();
    // Несколько строк X-Forwarded-For равнозначны одной, склеенной запятой. Строки берутся байтами:
    // значение с obs-text не `to_str()`, и клиент не должен этим выбивать записи прокси из той же строки.
    let forwarded_for = headers
        .get_all(FORWARDED_FOR)
        .iter()
        .map(HeaderValue::as_bytes)
        .collect::<Vec<_>>()
        .join(&b',');
    RequestContext {
        request_id: forwarded_id.unwrap_or_else(|| RequestId::from(Uuid::now_v7())),
        trace_id: text(headers, TRACEPARENT).and_then(trace::trace_id),
        client_ip: peer.map(|ip| proxies.client_ip(ip, &forwarded_for)),
        user_agent: user_agent(headers),
    }
}

fn echo_request_id(response: &mut Response, id: &RequestId) {
    if let Ok(value) = HeaderValue::from_str(id.as_str()) {
        response.headers_mut().insert(REQUEST_ID, value);
    }
}

/// Самый внешний слой `api_router`.
pub(crate) async fn request_context(
    State(proxies): State<Arc<TrustedProxies>>,
    mut request: Request,
    next: Next,
) -> Response {
    let started = Instant::now();
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| addr.ip());
    let context = context(peer, request.headers(), &proxies);
    let method = request.method().clone();
    // Только путь: в query бывают токены и персональные данные.
    let path = request.uri().path().to_owned();
    request.extensions_mut().insert(context.clone());

    let span = tracing::info_span!(
        "request",
        request_id = context.request_id.as_str(),
        trace_id = context.trace_id.as_deref(),
    );
    let response = next.run(request).instrument(span).await;
    let mut response = render::render_error(response, &context.request_id);
    echo_request_id(&mut response, &context.request_id);

    let actor = response
        .extensions()
        .get::<Actor>()
        .map(|actor| actor.0.as_str());
    let facts = response.extensions().get::<CacheFacts>();
    tracing::info!(
        target: ACCESS_TARGET,
        method = %method,
        path = %path,
        status = response.status().as_u16(),
        duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX),
        request_id = context.request_id.as_str(),
        client_ip = context.client_ip.map(tracing::field::display),
        user_agent = context.user_agent.as_deref(),
        actor,
        cache = facts.and_then(|facts| facts.cache),
        upstream_failure = facts.and_then(|facts| facts.failure),
        upstream_reason = facts.and_then(|facts| facts.reason.as_deref()),
        "запрос обработан"
    );
    response
}

/// Лёгкий слой для admin-listener: только тела ошибок и `X-Request-Id`.
/// Access-журнал пробам не нужен (оркестратор дёргает их каждые секунды), доверенных прокси там нет.
pub(crate) async fn render_only(request: Request, next: Next) -> Response {
    let id = RequestId::from(Uuid::now_v7());
    let response = next.run(request).await;
    let mut response = render::render_error(response, &id);
    echo_request_id(&mut response, &id);
    response
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(pairs: &[(&'static str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.append(
                *name,
                HeaderValue::from_str(value).expect("заголовок в тесте"),
            );
        }
        map
    }

    fn proxies() -> TrustedProxies {
        TrustedProxies::parse("10.0.0.0/8").expect("список в тесте")
    }

    #[test]
    fn request_id_is_taken_from_trusted_peer_only_when_valid() {
        // Arrange
        let trusted: Option<IpAddr> = "10.0.0.1".parse().ok();
        let stranger: Option<IpAddr> = "203.0.113.5".parse().ok();
        let proxies = proxies();
        let valid = headers(&[("x-request-id", "abc-1")]);
        let invalid = headers(&[("x-request-id", "has space")]);
        let cases = [
            (trusted, &valid),
            (stranger, &valid),
            (None, &valid),
            (trusted, &invalid),
        ];

        // Act
        let echoed = cases
            .map(|(peer, headers)| context(peer, headers, &proxies).request_id.as_str() == "abc-1");

        // Assert
        assert_eq!(echoed, [true, false, false, false]);
    }

    #[test]
    fn generated_request_id_is_uuid_v7() {
        // Arrange
        let headers = headers(&[("x-request-id", "abc-1")]);
        let proxies = proxies();

        // Act
        let ctx = context(None, &headers, &proxies);

        // Assert
        let parsed = Uuid::parse_str(ctx.request_id.as_str()).expect("uuid");
        assert_eq!(parsed.get_version_num(), 7);
    }

    #[test]
    fn user_agent_is_truncated_and_invalid_utf8_is_dropped() {
        // Arrange
        let long = "a".repeat(USER_AGENT_MAX_CHARS + 50);
        let long_headers = headers(&[("user-agent", long.as_str())]);
        let mut binary = HeaderMap::new();
        binary.insert(
            "user-agent",
            HeaderValue::from_bytes(&[0xff, 0xfe]).expect("байты"),
        );

        // Act
        let actual = [user_agent(&long_headers), user_agent(&binary)];

        // Assert
        assert_eq!(actual, [Some("a".repeat(USER_AGENT_MAX_CHARS)), None]);
    }

    #[test]
    fn client_ip_joins_several_forwarded_for_lines() {
        // Arrange
        let peer: Option<IpAddr> = "10.0.0.1".parse().ok();
        let headers = headers(&[
            ("x-forwarded-for", "6.6.6.6, 203.0.113.9"),
            ("x-forwarded-for", "10.0.0.2"),
        ]);
        let proxies = proxies();

        // Act
        let ctx = context(peer, &headers, &proxies);

        // Assert
        assert_eq!(ctx.client_ip, "203.0.113.9".parse().ok());
    }
}
