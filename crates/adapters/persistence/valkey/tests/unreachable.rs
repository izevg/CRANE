//! Недоступный Valkey: команды отказывают сразу, а не ждут `COMMAND_TIMEOUT`.

#![allow(clippy::expect_used, reason = "тесты: падение = провал проверки")]

use std::time::{Duration, Instant};

use crane_domain::proxy::{ProxyKey, UpstreamPath};
use crane_domain::registry::RepositoryId;
use crane_ports::proxy::ProxyCache;
use crane_valkey::{Endpoint, Valkey};
use uuid::Uuid;

/// Порт 1 никто не слушает: соединение отклоняется.
const CLOSED_PORT_URL: &str = "valkey://127.0.0.1:1";
const L1_BYTES: u64 = 1 << 20;
/// Заведомо меньше потолка команды в адаптере (1 с): без быстрого отказа команда ждала бы его.
const FAST: Duration = Duration::from_millis(500);

#[test]
fn command_to_an_unreachable_server_fails_without_waiting_for_the_command_timeout() {
    // Arrange
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("runtime тестов");
    let endpoint = Endpoint::parse(CLOSED_PORT_URL).expect("адрес валиден");
    let valkey = runtime.block_on(async { Valkey::start(endpoint, L1_BYTES) });
    let key = ProxyKey {
        repository: RepositoryId::new(Uuid::nil()),
        path: UpstreamPath::parse("index/se/rd/serde").expect("путь валиден"),
    };

    // Act
    let (failed, elapsed) = runtime.block_on(async {
        let started = Instant::now();
        (valkey.get(&key).await.is_err(), started.elapsed())
    });

    // Assert
    assert_eq!((failed, elapsed < FAST), (true, true));
}
