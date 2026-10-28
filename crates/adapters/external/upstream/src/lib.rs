//! Адаптер upstream-реестров: HTTP-клиент с SSRF guard.
//!
//! Guard стоит в двух местах, где reqwest принимает решения сам: DNS-резолвер проверяет каждый адрес до connect,
//! redirect-политика проверяет хост и схему каждого hop. Публичного способа обойти их нет.

mod guard;
mod headers;

use std::error::Error;
use std::fmt;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crane_domain::proxy::EgressPolicy;
use crane_ports::proxy::{Upstream, UpstreamError, UpstreamRequest, UpstreamResponse};
use futures_util::TryStreamExt;
use reqwest::dns::Resolve;
use reqwest::header::{ACCEPT, IF_NONE_MATCH};
use reqwest::redirect;
use reqwest::{Response, StatusCode};

use guard::{GuardedResolver, Hop, hop, refusal};
use headers::{Classified, classify};

/// Таймаут установки соединения: быстрый отказ недоступному upstream, чтобы не держать lease.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Таймаут тишины между чтениями. Общего таймаута нет сознательно: большой `.crate` легально качается долго,
/// а зависший поток обрывается по простою.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Интервал HTTP/2 PING к upstream. Меньше типичного таймаута простоя NAT и файрвола (Docker Desktop,
/// облачные NAT-шлюзы от ~60 с): они молча роняют простаивающее соединение, а без пингов hyper этого не видит
/// и мультиплексирует новые запросы в мёртвое соединение, где их не спасает даже `READ_TIMEOUT`.
const UPSTREAM_PING: Duration = Duration::from_secs(15);

/// Сколько ждать ответ на PING. Без ответа hyper закрывает соединение, и следующий запрос открывает новое.
const UPSTREAM_PING_TIMEOUT: Duration = Duration::from_secs(10);

/// Предел простоя соединения в пуле. Для HTTP/1.1 (static.crates.io), где PING нет, это единственная защита
/// от переиспользования соединения, которое NAT уже забыл; значение по умолчанию у reqwest (90 с) больше окна NAT.
const UPSTREAM_POOL_IDLE: Duration = Duration::from_secs(30);

/// Цель журнала egress (раздел 16.6): одна запись на каждую реальную выборку из upstream, включая отказы
/// SSRF guard. Без заголовков, тел и `ETag`: журнал для безопасников, а не отладочный дамп.
pub const EGRESS_TARGET: &str = "crane::upstream";

/// Политика crates.io требует идентифицируемый User-Agent.
const USER_AGENT: &str = concat!("crane/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, thiserror::Error)]
#[error("не удалось собрать HTTP-клиент upstream")]
pub struct BuildError(#[source] reqwest::Error);

/// HTTP-транспорт к upstream.
pub struct HttpUpstream {
    client: reqwest::Client,
}

// У клиента reqwest в Debug вся конфигурация, включая резолвер: наружу она не нужна.
impl fmt::Debug for HttpUpstream {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpUpstream").finish_non_exhaustive()
    }
}

impl HttpUpstream {
    pub fn new(policy: EgressPolicy) -> Result<Self, BuildError> {
        let resolver = GuardedResolver {
            policy: policy.clone(),
        };
        Self::build(policy, Arc::new(resolver))
    }

    // Резолвер подменяют только тесты (приватный вызов): публичной точки с чужим резолвером нет.
    // `ClientBuilder::resolve`/`resolve_to_addrs` не вызываем никогда: их переопределения идут в обход `dns_resolver`.
    fn build(policy: EgressPolicy, resolver: Arc<dyn Resolve>) -> Result<Self, BuildError> {
        let client = reqwest::Client::builder()
            .dns_resolver(resolver)
            .redirect(redirect::Policy::custom(move |attempt| {
                match hop(&policy, attempt.url(), attempt.previous().len()) {
                    Hop::Follow => attempt.follow(),
                    Hop::Stop => attempt.stop(),
                    Hop::Refuse(refusal) => attempt.error(refusal),
                }
            }))
            // Прокси из окружения увёл бы DNS-резолв за пределы guard: connect шёл бы к прокси.
            .no_proxy()
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            // Именно простаивающее в пуле соединение и рвёт NAT, поэтому пинги нужны и без активных потоков.
            .http2_keep_alive_interval(Some(UPSTREAM_PING))
            .http2_keep_alive_timeout(UPSTREAM_PING_TIMEOUT)
            .http2_keep_alive_while_idle(true)
            .pool_idle_timeout(Some(UPSTREAM_POOL_IDLE))
            .user_agent(USER_AGENT)
            .build()
            .map_err(BuildError)?;
        Ok(Self { client })
    }
}

fn unavailable(error: impl Into<Box<dyn Error + Send + Sync>>) -> UpstreamError {
    UpstreamError::Unavailable(error.into())
}

/// Пишет событие egress по уже классифицированному исходу. `path` — путь upstream без query;
/// `elapsed_ms` — время до получения заголовков.
fn record_egress(
    host: &str,
    path: &str,
    outcome: Result<StatusCode, &UpstreamError>,
    elapsed: Duration,
) {
    let elapsed_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
    match outcome {
        Ok(status) => tracing::info!(
            target: EGRESS_TARGET,
            host,
            path,
            status = status.as_u16(),
            elapsed_ms,
            "выборка из upstream"
        ),
        Err(error) => {
            let failure = match error {
                UpstreamError::Refused(_) => "refused",
                UpstreamError::Unavailable(_) => "unavailable",
                UpstreamError::Throttled => "throttled",
            };
            tracing::info!(
                target: EGRESS_TARGET,
                host,
                path,
                failure,
                elapsed_ms,
                "выборка из upstream"
            );
        }
    }
}

impl Upstream for HttpUpstream {
    async fn fetch(&self, request: UpstreamRequest<'_>) -> Result<UpstreamResponse, UpstreamError> {
        let url = request.base.url(request.path);
        let mut call = self.client.get(&url);
        if let Some(accept) = request.accept {
            call = call.header(ACCEPT, accept);
        }
        if let Some(etag) = request.if_none_match {
            call = call.header(IF_NONE_MATCH, etag.as_str());
        }
        let started = Instant::now();
        // Исход классифицируется один раз: по нему же пишется журнал egress.
        let sent = call.send().await.map_err(|error| {
            refusal(&error).map_or_else(|| unavailable(error), UpstreamError::Refused)
        });
        record_egress(
            request.base.host(),
            request.path.as_str(),
            sent.as_ref().map(Response::status),
            started.elapsed(),
        );
        let response = sent?;
        match classify(response.status(), response.headers()) {
            Classified::Found { etag, max_age } => Ok(UpstreamResponse::Found {
                body: Box::pin(response.bytes_stream().map_err(io::Error::other)),
                etag,
                max_age,
            }),
            Classified::NotModified { max_age } => Ok(UpstreamResponse::NotModified { max_age }),
            Classified::NotFound => Ok(UpstreamResponse::NotFound),
            Classified::Throttled => Err(UpstreamError::Throttled),
            Classified::Other(status) => {
                Err(unavailable(format!("upstream ответил статусом {status}")))
            }
        }
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
