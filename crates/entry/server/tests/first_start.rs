//! Первый запуск на настоящем бинарнике и PostgreSQL: двухфазный `serve` в одном процессе.
//!
//! `CRANE_TEST_DATABASE_URL` - URL существующей БД (на ней создаются отдельные БД под тест).
//! Без переменной тесты сообщают о пропуске, а при `CRANE_TEST_REQUIRE_DB=1` падают.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stderr,
    reason = "в тестах падение unwrap - это падение теста, а пропуск виден только в stderr"
)]

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::{Child, Command, Output, Stdio};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use crane_domain::access::Principal;
use crane_domain::onboarding::{Onboarding, TokenHash};
use crane_domain::settings::{NewValue, Secret};
use crane_memory::ManualClock;
use crane_pg::{PgOnboarding, PgSettings, SettingsKey};
use crane_ports::settings::{OnboardingStore, SettingsStore};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{AssertSqlSafe, PgPool};
use tokio::runtime::Runtime;
use url::Url;
use uuid::Uuid;

const URL_ENV: &str = "CRANE_TEST_DATABASE_URL";
const REQUIRE_ENV: &str = "CRANE_TEST_REQUIRE_DB";
const TOKEN: &str = "first-start-token";
const KEY_A: &str = "AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8=";
const KEY_B: &str = "Hh8dHBsaGRgXFhUUExIREA8ODQwLCgkIBwYFBAMCAQA=";
const START_TIMEOUT: Duration = Duration::from_secs(30);
const PHASE_SWITCH_TIMEOUT: Duration = Duration::from_secs(20);

fn database_url() -> Option<String> {
    let url = std::env::var(URL_ENV).ok();
    if url.is_none() {
        assert!(
            std::env::var(REQUIRE_ENV).as_deref() != Ok("1"),
            "{REQUIRE_ENV}=1, но {URL_ENV} не задана"
        );
        eprintln!("ПРОПУСК: {URL_ENV} не задана, тесты запуска не выполняются");
    }
    url
}

/// Отдельная БД на тест: строки `setting` и `onboarding` общей базы мешали бы друг другу.
struct Stand {
    runtime: Runtime,
    admin: PgPool,
    name: String,
    url: String,
    dir: tempfile::TempDir,
}

impl Stand {
    fn create(base: &str) -> Self {
        let runtime = Runtime::new().unwrap();
        let name = format!("crane_first_start_{}", Uuid::new_v4().simple());
        let admin = runtime
            .block_on(PgPoolOptions::new().connect_with(PgConnectOptions::from_str(base).unwrap()))
            .unwrap();
        runtime
            .block_on(sqlx::query(AssertSqlSafe(format!("CREATE DATABASE {name}"))).execute(&admin))
            .unwrap();
        let mut url = Url::parse(base).unwrap();
        url.set_path(&format!("/{name}"));
        let stand = Self {
            runtime,
            admin,
            name,
            url: url.to_string(),
            dir: tempfile::tempdir().unwrap(),
        };
        let migrated = stand.command(&[], &[], "migrate").output().unwrap();
        assert!(migrated.status.success(), "{migrated:?}");
        stand
    }

    fn command(&self, env: &[(&str, &str)], ports: &[(&str, u16)], role: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_crane"));
        command
            .arg(role)
            .env_clear()
            .env("CRANE_DATABASE_URL", &self.url)
            .env(
                "CRANE_BLOBS",
                format!("file://{}", self.dir.path().display()),
            )
            .env("CRANE_PUBLIC_URL", "https://crane.example")
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        for (name, value) in env {
            command.env(name, value);
        }
        for (name, port) in ports {
            command.env(name, port.to_string());
        }
        command
    }

    fn pool(&self) -> PgPool {
        let options = PgConnectOptions::from_str(&self.url).unwrap();
        self.runtime
            .block_on(PgPoolOptions::new().connect_with(options))
            .unwrap()
    }

    fn clock() -> Arc<ManualClock> {
        Arc::new(ManualClock::new(SystemTime::now()))
    }

    fn onboarding(&self) -> PgOnboarding {
        PgOnboarding::new(self.pool(), Self::clock())
    }

    fn begin(&self, state: Onboarding) {
        self.runtime
            .block_on(self.onboarding().begin(state))
            .unwrap();
    }

    fn seal_token_with(&self, key: &str) {
        let key = SettingsKey::from_base64(&Secret::new(key.to_owned())).unwrap();
        let store = PgSettings::new(self.pool(), Self::clock(), Some(key));
        let by = Principal::system();
        let value = NewValue::Secret(Secret::new("stored-token".to_owned()));
        self.runtime
            .block_on(store.put("token", value, None, &by))
            .unwrap();
    }
}

impl Drop for Stand {
    fn drop(&mut self) {
        let drop_database = format!("DROP DATABASE IF EXISTS {} WITH (FORCE)", self.name);
        let _ = self
            .runtime
            .block_on(sqlx::query(AssertSqlSafe(drop_database)).execute(&self.admin));
    }
}

fn completed() -> Onboarding {
    Onboarding::Completed {
        at: SystemTime::now(),
        by: "crane:system".to_owned(),
    }
}

fn pending() -> Onboarding {
    Onboarding::Pending {
        token: TokenHash::of(b"crb_seeded"),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
    }
}

fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// Ответ как есть: строка статуса, заголовки и тело в нижнем регистре заголовков не нормализуются.
fn get(port: u16, path: &str) -> Option<(u16, String)> {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: crane\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).ok()?;
    let status = reply.split_whitespace().nth(1)?.parse().ok()?;
    Some((status, reply))
}

fn wait_until(what: &str, mut ready: impl FnMut() -> bool, limit: Duration) {
    let started = Instant::now();
    while !ready() {
        assert!(started.elapsed() < limit, "не дождались: {what}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn status_of(port: u16, path: &str) -> Option<u16> {
    get(port, path).map(|(status, _)| status)
}

struct Server {
    child: Child,
    http: u16,
    admin: u16,
}

impl Server {
    fn start(stand: &Stand, env: &[(&str, &str)]) -> Self {
        let (http, admin) = (free_port(), free_port());
        let ports = [("CRANE_HTTP_PORT", http), ("CRANE_ADMIN_PORT", admin)];
        let child = stand.command(env, &ports, "serve").spawn().unwrap();
        let server = Self { child, http, admin };
        wait_until(
            "healthz/ready 200",
            || status_of(admin, "/healthz/ready") == Some(200),
            START_TIMEOUT,
        );
        server
    }

    fn stop(mut self) -> Output {
        self.child.kill().unwrap();
        self.child.wait_with_output().unwrap()
    }
}

fn token_lines(output: &Output) -> usize {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains("bootstrap token: crb_"))
        .count()
}

#[test]
fn completed_wizard_with_unreadable_token_exits_nonzero_naming_the_key_and_both_fingerprints() {
    // Arrange
    let Some(base) = database_url() else { return };
    let stand = Stand::create(&base);
    stand.seal_token_with(KEY_A);
    stand.begin(completed());
    let fingerprint = |key: &str| {
        SettingsKey::from_base64(&Secret::new(key.to_owned()))
            .unwrap()
            .id()
            .to_owned()
    };

    // Act
    let output = stand
        .command(&[("CRANE_SETTINGS_KEY", KEY_B)], &[], "serve")
        .output()
        .unwrap();

    // Assert
    let log = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "{log}");
    assert!(log.contains("`token`"), "{log}");
    assert!(log.contains(&fingerprint(KEY_A)), "{log}");
    assert!(log.contains(&fingerprint(KEY_B)), "{log}");
}

/// Мастер открыт, протоколы отвечают 503, затем мастер завершён строкой в БД: тот же процесс
/// на тех же портах входит в рабочий режим. Возвращает статусы протокола и пробы до и после.
fn observe_phase_switch(stand: &Stand) -> [Option<u16>; 4] {
    let server = Server::start(stand, &[("CRANE_TOKEN", TOKEN)]);
    let protocol = "/cargo/main/api/v1/crates/some";
    let before = (
        status_of(server.http, protocol),
        status_of(server.admin, "/healthz/ready"),
    );
    stand
        .runtime
        .block_on(stand.onboarding().complete(&Principal::system()))
        .unwrap();
    wait_until(
        "рабочий режим на публичном порту",
        || status_of(server.http, protocol) == Some(401),
        PHASE_SWITCH_TIMEOUT,
    );
    let after = (
        status_of(server.http, protocol),
        status_of(server.admin, "/healthz/ready"),
    );
    server.stop();
    [before.0, before.1, after.0, after.1]
}

#[test]
fn wizard_phase_yields_to_working_phase_in_the_same_process_when_the_row_completes() {
    // Arrange
    let Some(base) = database_url() else { return };
    let stand = Stand::create(&base);
    stand.begin(pending());

    // Act
    let statuses = observe_phase_switch(&stand);

    // Assert
    assert_eq!(statuses, [Some(503), Some(200), Some(401), Some(200)]);
}

/// Первый запуск без конфигурации под `RUST_LOG=warn`, затем рестарт. Возвращает число строк
/// с токеном в первом и втором запуске и ответ протокола первого.
fn first_run_then_restart(stand: &Stand) -> (usize, usize, Option<(u16, String)>) {
    let env = [("RUST_LOG", "warn")];
    let first = Server::start(stand, &env);
    let protocol = get(first.http, "/npm/registry/left-pad");
    let printed = token_lines(&first.stop());
    let second = Server::start(stand, &env);
    (printed, token_lines(&second.stop()), protocol)
}

#[test]
fn first_start_without_configuration_prints_the_token_once_and_a_restart_does_not() {
    // Arrange
    let Some(base) = database_url() else { return };
    let stand = Stand::create(&base);

    // Act
    let (first, restart, protocol) = first_run_then_restart(&stand);

    // Assert
    let (status, reply) = protocol.unwrap();
    assert_eq!((first, restart, status), (1, 0, 503));
    assert!(reply.to_lowercase().contains("retry-after: 10"), "{reply}");
    assert!(reply.contains("not-configured"), "{reply}");
}

#[test]
fn onboarding_token_role_reports_a_first_start_that_has_not_begun() {
    // Arrange
    let Some(base) = database_url() else { return };
    let stand = Stand::create(&base);

    // Act
    let output = stand
        .command(&[], &[], "onboarding-token")
        .output()
        .unwrap();

    // Assert
    let log = String::from_utf8_lossy(&output.stdout);
    assert!(!output.status.success(), "{log}");
    assert!(log.contains("первый запуск ещё не начат"), "{log}");
}

/// Сброс без флага и с ним. Возвращает оба вывода и состояние строки после каждого.
fn reset_twice(stand: &Stand) -> [(Output, bool); 2] {
    let attempt = |args: &[&str]| {
        let mut command = stand.command(&[], &[], args[0]);
        command.args(&args[1..]);
        let output = command.output().unwrap();
        let state = stand.runtime.block_on(stand.onboarding().get()).unwrap();
        (output, matches!(state, Some(Onboarding::Pending { .. })))
    };
    [
        attempt(&["onboarding-reset"]),
        attempt(&["onboarding-reset", "--confirm"]),
    ]
}

#[test]
fn onboarding_reset_refuses_without_confirm_and_with_it_reopens_the_wizard_printing_a_token() {
    // Arrange
    let Some(base) = database_url() else { return };
    let stand = Stand::create(&base);
    stand.begin(completed());

    // Act
    let [
        (refused, pending_after_refusal),
        (confirmed, pending_after_confirm),
    ] = reset_twice(&stand);

    // Assert
    let refusal_log = String::from_utf8_lossy(&refused.stdout);
    assert!(refusal_log.contains("--confirm"), "{refusal_log}");
    assert_eq!(
        (
            refused.status.success(),
            pending_after_refusal,
            confirmed.status.success(),
            pending_after_confirm,
            token_lines(&confirmed)
        ),
        (false, false, true, true, 1)
    );
}
