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
            accept: None,
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
            accept: None,
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
            accept: None,
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

#[tokio::test]
async fn accept_header_is_sent_when_requested() {
    // Arrange
    let (listener, port) = listener().await;
    let server = serve_once(
        listener,
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
    );
    let policy = EgressPolicy::new([HOST], true);
    let base = policy.admit("http", HOST, Some(port), "").expect("admit");
    let upstream = HttpUpstream::build(policy, Arc::new(Loopback)).expect("client");
    let path = UpstreamPath::parse("simple/x/").expect("path");

    // Act
    let result = upstream
        .fetch(UpstreamRequest {
            base: &base,
            path: &path,
            accept: Some("application/vnd.pypi.simple.v1+json"),
            if_none_match: None,
        })
        .await;

    // Assert
    let request = server.await.expect("server").to_ascii_lowercase();
    assert!(matches!(result, Ok(UpstreamResponse::NotFound)));
    assert!(request.starts_with("get /simple/x/ "));
    assert!(request.contains("accept: application/vnd.pypi.simple.v1+json\r\n"));
}

#[tokio::test]
async fn accept_header_is_left_to_the_client_default_when_not_requested() {
    // Arrange
    let (listener, port) = listener().await;
    let server = serve_once(
        listener,
        "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
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
            accept: None,
            if_none_match: None,
        })
        .await;

    // Assert
    let request = server.await.expect("server").to_ascii_lowercase();
    assert!(matches!(result, Ok(UpstreamResponse::NotFound)));
    assert!(request.contains("accept: */*\r\n"));
}
