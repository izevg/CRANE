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
use reqwest::header::IF_NONE_MATCH;
use reqwest::redirect;
use reqwest::{Response, StatusCode};

use guard::{GuardedResolver, Hop, hop, refusal};
use headers::{Classified, classify};

/// Таймаут установки соединения: быстрый отказ недоступному upstream, чтобы не держать lease.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// Таймаут тишины между чтениями. Общего таймаута нет сознательно: большой `.crate` легально качается долго,
/// а зависший поток обрывается по простою.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

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
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};

    use crane_domain::proxy::{EgressRefusal, UpstreamPath};
    use crane_ports::proxy::ETag;
    use futures_util::{FutureExt, StreamExt};
    use reqwest::dns::{Addrs, Name, Resolving};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio::task::JoinHandle;

    use super::*;

    const HOST: &str = "upstream.test";

    /// Любое имя → loopback: проверок guard нет, тест полагается на redirect-политику.
    #[derive(Debug)]
    struct Loopback;

    impl Resolve for Loopback {
        fn resolve(&self, _name: Name) -> Resolving {
            Box::pin(async {
                let addrs: Addrs =
                    Box::new([SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 0)].into_iter());
                Ok(addrs)
            })
        }
    }

    async fn listener() -> (TcpListener, u16) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .await
            .expect("bind");
        let port = listener.local_addr().expect("addr").port();
        (listener, port)
    }

    /// Принимает одно соединение, возвращает текст запроса, отвечает заданным сырым ответом.
    fn serve_once(listener: TcpListener, response: &'static str) -> JoinHandle<String> {
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = socket.read(&mut chunk).await.expect("read");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
            }
            socket.write_all(response.as_bytes()).await.expect("write");
            socket.shutdown().await.expect("shutdown");
            String::from_utf8_lossy(&request).into_owned()
        })
    }

    /// Тело (собранное из потока), `ETag` и срок свежести ответа 200; для любого другого исхода `None`.
    async fn found_parts(
        result: Result<UpstreamResponse, UpstreamError>,
    ) -> Option<(Vec<u8>, Option<ETag>, Option<Duration>)> {
        match result {
            Ok(UpstreamResponse::Found {
                body,
                etag,
                max_age,
            }) => {
                let chunks = body.map(|chunk| chunk.expect("chunk").to_vec());
                Some((chunks.concat().await, etag, max_age))
            }
            _ => None,
        }
    }

    #[tokio::test]
    async fn production_resolver_refuses_non_public_address_without_connecting() {
        // Arrange
        let (listener, port) = listener().await;
        let policy = EgressPolicy::new(["localhost"], true);
        let base = policy
            .admit("http", "localhost", Some(port), "")
            .expect("admit");
        let upstream = HttpUpstream::new(policy).expect("client");
        let path = UpstreamPath::parse("index/se/rd/serde").expect("path");

        // Act
        let result = upstream
            .fetch(UpstreamRequest {
                base: &base,
                path: &path,
                if_none_match: None,
            })
            .await;

        // Assert
        let connected = listener.accept().now_or_never().is_some();
        assert!(matches!(
            result,
            Err(UpstreamError::Refused(EgressRefusal::NonPublicAddress(_)))
        ));
        assert!(!connected, "к listener не должно быть ни одного соединения");
    }

    #[tokio::test]
    async fn redirect_to_foreign_host_is_refused() {
        // Arrange
        let (listener, port) = listener().await;
        let server = serve_once(
            listener,
            "HTTP/1.1 302 Found\r\nLocation: http://evil.example/x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        let policy = EgressPolicy::new([HOST], true);
        let base = policy.admit("http", HOST, Some(port), "").expect("admit");
        let upstream = HttpUpstream::build(policy, Arc::new(Loopback)).expect("client");
        let path = UpstreamPath::parse("a/b").expect("path");

        // Act
        let result = upstream
            .fetch(UpstreamRequest {
                base: &base,
                path: &path,
                if_none_match: None,
            })
            .await;

        // Assert
        server.await.expect("server");
        assert!(matches!(
            result,
            Err(UpstreamError::Refused(EgressRefusal::HostNotAllowed(host))) if host == "evil.example"
        ));
    }

    #[tokio::test]
    async fn ok_response_carries_body_etag_and_remaining_max_age_and_sends_validator() {
        // Arrange
        let (listener, port) = listener().await;
        let server = serve_once(
            listener,
            "HTTP/1.1 200 OK\r\nETag: \"v2\"\r\nCache-Control: max-age=600\r\nAge: 100\r\nContent-Length: 5\r\nConnection: close\r\n\r\nhello",
        );
        let policy = EgressPolicy::new([HOST], true);
        let base = policy.admit("http", HOST, Some(port), "").expect("admit");
        let upstream = HttpUpstream::build(policy, Arc::new(Loopback)).expect("client");
        let path = UpstreamPath::parse("a/b").expect("path");
        let validator = ETag::new("\"v1\"".to_owned());

        // Act
        let result = upstream
            .fetch(UpstreamRequest {
                base: &base,
                path: &path,
                if_none_match: Some(&validator),
            })
            .await;

        // Assert
        let request = server.await.expect("server").to_ascii_lowercase();
        assert_eq!(
            found_parts(result).await,
            Some((
                b"hello".to_vec(),
                Some(ETag::new("\"v2\"".to_owned())),
                Some(Duration::from_secs(500))
            ))
        );
        assert!(request.contains("if-none-match: \"v1\""));
    }
}
