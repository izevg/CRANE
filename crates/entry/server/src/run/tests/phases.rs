//! Цикл фаз `serve` над настоящей БД: что делает старт при каждом состоянии `onboarding`.
//! Сокетов нет: проверяется решение `advance`, а не слушатели.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap - это падение теста"
)]

use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crane_domain::audit::AuditAction;
use crane_domain::onboarding::{Onboarding, TokenHash};
use crane_domain::settings::{NewValue, Secret};
use crane_memory::{MemoryAudit, SequentialIdGen};
use crane_pg::PgOnboarding;
use crane_ports::settings::OnboardingStore;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};

use super::*;
use crate::onboarding::Journal;
use crate::run::settings_tests::{FreshDb, KEY_A, KEY_B, key_id};
use crate::run::stack::{TOKEN, database_url, lookup, runtime, token_lookup};

const SYSTEM: &str = "crane:system";

fn phases(db: &FreshDb, boot: Boot, audit: &Arc<MemoryAudit>) -> Phases {
    let backend = db.runtime.block_on(Backend::open(&boot)).unwrap();
    Phases {
        boot,
        backend,
        journal: Journal::new(audit.clone(), Arc::new(SequentialIdGen::default())),
    }
}

fn onboarding(db: &FreshDb) -> PgOnboarding {
    let options = PgConnectOptions::from_str(&db.url).unwrap();
    let pool = db
        .runtime
        .block_on(PgPoolOptions::new().connect_with(options))
        .unwrap();
    PgOnboarding::new(pool, Arc::new(SystemClock))
}

fn completed() -> Onboarding {
    Onboarding::Completed {
        at: SystemTime::now(),
        by: SYSTEM.to_owned(),
    }
}

fn pending() -> Onboarding {
    Onboarding::Pending {
        token: TokenHash::of(b"crb_old"),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
    }
}

fn kind(next: &Result<Next, RunError>) -> &'static str {
    match next {
        Ok(Next::Again) => "again",
        Ok(Next::Wizard) => "wizard",
        Ok(Next::Ready(..)) => "ready",
        Err(_) => "error",
    }
}

fn state_label(state: Option<&Onboarding>) -> String {
    match state {
        None => "none".to_owned(),
        Some(Onboarding::Pending { .. }) => "pending".to_owned(),
        Some(Onboarding::Completed { by, .. }) => format!("completed:{by}"),
    }
}

#[test]
fn completed_wizard_with_unreadable_token_stops_the_start_naming_the_key_and_both_fingerprints() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    db.put(
        &db.store(Some(KEY_A)),
        "token",
        NewValue::Secret(Secret::new("stored-token".to_owned())),
    );
    db.runtime
        .block_on(onboarding(&db).begin(completed()))
        .unwrap();
    let boot = lookup(&db.url, dir.path(), &[("CRANE_SETTINGS_KEY", KEY_B)]);
    let phases = phases(&db, boot, &Arc::new(MemoryAudit::default()));

    // Act
    let next = db.runtime.block_on(phases.advance());

    // Assert
    let message = next.err().map(|e| e.to_string()).unwrap_or_default();
    db.drop_database();
    assert!(message.contains("`token`"), "{message}");
    assert!(message.contains(&key_id(KEY_A)), "{message}");
    assert!(message.contains(&key_id(KEY_B)), "{message}");
}

#[test]
fn completed_wizard_with_missing_configuration_stops_the_start_naming_the_variable() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    db.runtime
        .block_on(onboarding(&db).begin(completed()))
        .unwrap();
    let phases = phases(
        &db,
        lookup(&db.url, dir.path(), &[]),
        &Arc::new(MemoryAudit::default()),
    );

    // Act
    let next = db.runtime.block_on(phases.advance());

    // Assert
    let message = next.err().map(|e| e.to_string());
    db.drop_database();
    assert_eq!(message, Some("не задана переменная CRANE_TOKEN".to_owned()));
}

#[test]
fn empty_install_without_configuration_opens_the_wizard_instead_of_failing() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    let audit = Arc::new(MemoryAudit::default());
    let phases = phases(&db, lookup(&db.url, dir.path(), &[]), &audit);

    // Act
    let passes = db
        .runtime
        .block_on(async { [phases.advance().await, phases.advance().await] });

    // Assert
    let state = db.runtime.block_on(onboarding(&db).get()).unwrap();
    let events = audit
        .events()
        .into_iter()
        .map(|e| e.action)
        .collect::<Vec<_>>();
    db.drop_database();
    assert_eq!(
        (
            passes.each_ref().map(kind),
            state_label(state.as_ref()),
            events
        ),
        (
            ["again", "wizard"],
            "pending".to_owned(),
            vec![AuditAction::OnboardingStarted]
        )
    );
}

#[test]
fn install_that_already_parses_is_marked_preconfigured_and_runs() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    let audit = Arc::new(MemoryAudit::default());
    let phases = phases(&db, token_lookup(&db.url, dir.path()), &audit);

    // Act
    let passes = db
        .runtime
        .block_on(async { [phases.advance().await, phases.advance().await] });

    // Assert
    let state = db.runtime.block_on(onboarding(&db).get()).unwrap();
    let events = audit
        .events()
        .into_iter()
        .map(|e| e.action)
        .collect::<Vec<_>>();
    db.drop_database();
    assert_eq!(
        (
            passes.each_ref().map(kind),
            state_label(state.as_ref()),
            events
        ),
        (
            ["again", "ready"],
            format!("completed:{SYSTEM}"),
            vec![AuditAction::OnboardingSkippedPreconfigured]
        )
    );
}

#[test]
fn pending_wizard_stays_in_the_wizard_even_when_the_configuration_parses() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    db.runtime
        .block_on(onboarding(&db).begin(pending()))
        .unwrap();
    let phases = phases(
        &db,
        lookup(&db.url, dir.path(), &[("CRANE_TOKEN", TOKEN)]),
        &Arc::new(MemoryAudit::default()),
    );

    // Act
    let next = db.runtime.block_on(phases.advance());

    // Assert
    db.drop_database();
    assert_eq!(kind(&next), "wizard");
}

#[test]
fn completed_wizard_with_valid_configuration_runs() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    db.runtime
        .block_on(onboarding(&db).begin(completed()))
        .unwrap();
    let phases = phases(
        &db,
        token_lookup(&db.url, dir.path()),
        &Arc::new(MemoryAudit::default()),
    );

    // Act
    let next = db.runtime.block_on(phases.advance());

    // Assert
    db.drop_database();
    assert_eq!(kind(&next), "ready");
}

#[test]
fn reset_of_a_running_install_returns_the_loop_to_the_wizard() {
    // Arrange
    let Some(base) = database_url() else { return };
    let db = FreshDb::create(&base);
    let dir = tempfile::tempdir().unwrap();
    let store = onboarding(&db);
    db.runtime.block_on(store.begin(completed())).unwrap();
    db.runtime
        .block_on(store.reset(
            TokenHash::of(b"crb_new"),
            SystemTime::now() + Duration::from_secs(60),
        ))
        .unwrap();
    let phases = phases(
        &db,
        token_lookup(&db.url, dir.path()),
        &Arc::new(MemoryAudit::default()),
    );

    // Act
    let next = db.runtime.block_on(phases.advance());

    // Assert
    db.drop_database();
    assert_eq!(kind(&next), "wizard");
}

#[test]
fn memory_install_has_no_wizard_and_decides_by_parsing_alone() {
    // Arrange
    let runtime = runtime();
    let dir = tempfile::tempdir().unwrap();
    let blobs = [("CRANE_BLOBS", "memory")];
    let broken = lookup("memory", dir.path(), &blobs);
    let valid = lookup("memory", dir.path(), &[blobs[0], ("CRANE_TOKEN", TOKEN)]);
    let audit = Arc::new(MemoryAudit::default());
    let build = |boot: Boot| Phases {
        backend: runtime.block_on(Backend::open(&boot)).unwrap(),
        boot,
        journal: Journal::new(audit.clone(), Arc::new(SequentialIdGen::default())),
    };
    let (broken, valid) = (build(broken), build(valid));

    // Act
    let passes = runtime.block_on(async { [broken.advance().await, valid.advance().await] });

    // Assert
    assert_eq!(
        (passes.each_ref().map(kind), audit.events().len()),
        (["error", "ready"], 0)
    );
}
