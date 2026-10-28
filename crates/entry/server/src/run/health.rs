//! Роль `healthcheck`: проба `/healthz/ready` для healthcheck в compose (в образе нет curl).

use std::time::Duration;

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::runtime::Builder;
use tokio::time::timeout;

use super::RunError;

const READY_PATH: &str = "/healthz/ready";
/// Таймаут compose — 3 с; проба обязана уложиться раньше, иначе причину увидит только оркестратор.
const PROBE_TIMEOUT: Duration = Duration::from_secs(2);
/// Строке статуса хватает пары десятков байт; предел защищает от бесконечного ответа.
const STATUS_LINE_LIMIT: u64 = 128;

pub(crate) fn healthcheck(admin_port: u16) -> Result<(), RunError> {
    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(probe(admin_port))
}

async fn probe(port: u16) -> Result<(), RunError> {
    let status = timeout(PROBE_TIMEOUT, status_line(port))
        .await
        .map_err(|_| format!("проба порта {port} не уложилась в {PROBE_TIMEOUT:?}"))??;
    if is_healthy(&status) {
        Ok(())
    } else {
        Err(format!("порт {port} ответил нездоровым статусом: {status}").into())
    }
}

async fn status_line(port: u16) -> Result<String, RunError> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
    let request =
        format!("GET {READY_PATH} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).await?;
    let mut line = String::new();
    BufReader::new(stream.take(STATUS_LINE_LIMIT))
        .read_line(&mut line)
        .await?;
    Ok(line.trim_end().to_owned())
}

/// Код сравнивается токеном целиком: `HTTP/1.1 2000` здоровым не считается.
fn is_healthy(status_line: &str) -> bool {
    let mut parts = status_line.split_whitespace();
    matches!(parts.next(), Some("HTTP/1.1" | "HTTP/1.0")) && parts.next() == Some("200")
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
    use std::sync::Arc;

    use crane_app::registry::Registry;
    use crane_http::admin_router;
    use crane_memory::{MemoryAudit, MemoryBlobStore, MemoryCatalog};
    use crane_ports::proxy::NoUpstream;
    use tokio::net::TcpListener;

    use super::*;
    use crate::system::{SystemClock, UuidV7Ids};

    /// Админ-роутер над хранилищами в памяти: проба готовности отвечает 200.
    fn admin() -> axum::Router {
        admin_router(Arc::new(Registry::new(
            MemoryCatalog::default(),
            MemoryBlobStore::default(),
            Arc::new(SystemClock),
            Arc::new(UuidV7Ids),
            Arc::new(MemoryAudit::default()),
            Arc::new(NoUpstream),
        )))
    }

    #[test]
    fn only_exact_200_over_http_1_is_healthy() {
        // Arrange
        let cases = [
            ("HTTP/1.1 200 OK", true),
            ("HTTP/1.0 200 OK", true),
            ("HTTP/1.1 200", true),
            ("HTTP/1.1 2000 OK", false),
            ("HTTP/1.1 503 Service Unavailable", false),
            ("HTTP/1.1 404 Not Found", false),
            ("HTTP/2 200", false),
            ("200 OK", false),
            ("garbage", false),
            ("", false),
        ];

        // Act
        let verdicts = cases.map(|(line, _)| is_healthy(line));

        // Assert
        assert_eq!(verdicts, cases.map(|(_, expected)| expected));
    }

    fn runtime() -> tokio::runtime::Runtime {
        Builder::new_current_thread().enable_all().build().unwrap()
    }

    #[test]
    fn probe_of_running_admin_listener_is_healthy() {
        // Arrange
        let runtime = runtime();

        // Act
        let result = runtime.block_on(async {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let port = listener.local_addr().unwrap().port();
            tokio::spawn(async move { axum::serve(listener, admin()).await });
            probe(port).await
        });

        // Assert
        assert!(result.is_ok(), "{result:?}");
    }

    #[test]
    fn healthcheck_of_closed_port_is_unhealthy() {
        // Arrange
        let port = runtime().block_on(async {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            listener.local_addr().unwrap().port()
        });

        // Act
        let result = healthcheck(port);

        // Assert
        assert!(result.is_err());
    }
}
