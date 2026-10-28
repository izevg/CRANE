#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap - это падение теста"
)]

use std::cell::RefCell;
use std::fmt::{self, Write as _};
use std::sync::Once;
use std::time::SystemTime;

use crane_memory::{ManualClock, MemoryAudit, MemoryOnboarding, SequentialIdGen};
use crane_siem::{AUDIT_TARGET, OcsfLogSink};
use tokio::runtime::Builder;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, SubscriberExt};

use super::*;

pub(in crate::onboarding) const NOW: SystemTime = SystemTime::UNIX_EPOCH;

pub(in crate::onboarding) fn run<F: Future>(future: F) -> F::Output {
    Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(future)
}

pub(in crate::onboarding) fn clock() -> Arc<ManualClock> {
    Arc::new(ManualClock::new(NOW))
}

pub(in crate::onboarding) fn store(clock: &Arc<ManualClock>) -> MemoryOnboarding {
    MemoryOnboarding::new(clock.clone())
}

pub(in crate::onboarding) fn journal(audit: &Arc<MemoryAudit>) -> Journal {
    Journal::new(audit.clone(), Arc::new(SequentialIdGen::default()))
}

/// Кто и что записал в журнал, без идентификаторов запросов.
pub(in crate::onboarding) fn facts(audit: &MemoryAudit) -> Vec<(String, AuditAction)> {
    audit
        .events()
        .into_iter()
        .map(|event| (event.actor.unwrap().id, event.action))
        .collect()
}

#[test]
fn decide_follows_the_phase_table() {
    // Arrange
    let cases = [
        (Row::Missing, true, false),
        (Row::Missing, false, false),
        (Row::Pending, true, false),
        (Row::Pending, false, false),
        (Row::Completed, true, false),
        (Row::Completed, false, false),
        (Row::Missing, true, true),
        (Row::Missing, false, true),
        (Row::Completed, true, true),
        (Row::Completed, false, true),
    ];

    // Act
    let steps = cases.map(|(row, parsed_ok, memory)| decide(row, parsed_ok, memory));

    // Assert
    assert_eq!(
        steps,
        [
            Step::Preconfigure,
            Step::StartWizard,
            Step::Wizard,
            Step::Wizard,
            Step::Ready,
            Step::Fail,
            Step::Ready,
            Step::Fail,
            Step::Ready,
            Step::Fail,
        ]
    );
}

#[test]
fn preconfigure_completes_by_system_and_records_the_skip_once() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let audit = Arc::new(MemoryAudit::default());

    // Act
    let state = run(async {
        preconfigure(&store, clock.as_ref(), &journal(&audit))
            .await
            .unwrap();
        store.get().await.unwrap().unwrap()
    });

    // Assert
    let completed =
        matches!(&state, Onboarding::Completed { at, by } if *at == NOW && by == "crane:system");
    assert_eq!(
        (completed, facts(&audit)),
        (
            true,
            vec![(
                "crane:system".to_owned(),
                AuditAction::OnboardingSkippedPreconfigured
            )]
        )
    );
}

#[test]
fn preconfigure_leaves_a_wizard_opened_by_another_replica_and_stays_silent() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let audit = Arc::new(MemoryAudit::default());
    let pending = Onboarding::Pending {
        token: crane_domain::onboarding::TokenHash::of(b"crb_other"),
        expires_at: NOW + bootstrap::TOKEN_TTL,
    };
    run(store.begin(pending)).unwrap();

    // Act
    let state = run(async {
        preconfigure(&store, clock.as_ref(), &journal(&audit))
            .await
            .unwrap();
        store.get().await.unwrap().unwrap()
    });

    // Assert
    assert_eq!(
        (matches!(state, Onboarding::Pending { .. }), facts(&audit)),
        (true, vec![])
    );
}

#[test]
fn start_wizard_opens_pending_and_only_the_winner_records_started() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let audit = Arc::new(MemoryAudit::default());

    // Act
    let state = run(async {
        for _ in 0..2 {
            start_wizard(&store, clock.as_ref(), &journal(&audit), Source::Generate)
                .await
                .unwrap();
        }
        store.get().await.unwrap().unwrap()
    });

    // Assert
    assert_eq!(
        (matches!(state, Onboarding::Pending { .. }), facts(&audit)),
        (
            true,
            vec![("crane:system".to_owned(), AuditAction::OnboardingStarted)]
        )
    );
}

#[test]
fn refresh_token_rotates_expired_token_and_records_the_rotation() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let audit = Arc::new(MemoryAudit::default());
    run(start_wizard(
        &store,
        clock.as_ref(),
        &journal(&audit),
        Source::Static,
    ))
    .unwrap();
    clock.advance(bootstrap::TOKEN_TTL);
    let state = run(store.get()).unwrap();

    // Act
    run(refresh_token(
        &store,
        clock.as_ref(),
        &journal(&audit),
        state.as_ref(),
        Source::Generate,
    ))
    .unwrap();

    // Assert
    let kinds = facts(&audit)
        .into_iter()
        .map(|(_, action)| action)
        .collect::<Vec<_>>();
    assert_eq!(
        kinds,
        vec![
            AuditAction::OnboardingStarted,
            AuditAction::OnboardingTokenRotated
        ]
    );
}

#[test]
fn until_completed_returns_once_the_row_is_completed() {
    // Arrange
    let clock = clock();
    let store = Arc::new(store(&clock));
    let pending = Onboarding::Pending {
        token: crane_domain::onboarding::TokenHash::of(b"crb_t"),
        expires_at: NOW + bootstrap::TOKEN_TTL,
    };
    run(store.begin(pending)).unwrap();

    // Act
    let waited = run(async {
        let finisher = store.clone();
        let finish = async {
            tokio::time::sleep(Duration::from_millis(30)).await;
            finisher.complete(&Principal::system()).await.unwrap();
        };
        let wait = until_completed(store.clone(), Duration::from_millis(5));
        let timeout = tokio::time::timeout(
            Duration::from_secs(5),
            futures_util::future::join(wait, finish),
        );
        timeout.await.is_ok()
    });

    // Assert
    assert!(waited);
}

type Captured = Vec<(String, String)>;

thread_local! {
    /// События текущего перехвата на этом потоке; `None` - вне перехвата.
    static CAPTURE: RefCell<Option<Captured>> = const { RefCell::new(None) };
}

/// События tracing: цель и поля одной строкой, в буфер перехвата текущего потока.
struct Collector;

struct Fields<'a>(&'a mut String);

impl Visit for Fields<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        write!(self.0, "{}={value:?};", field.name()).ok();
    }
}

impl<S: Subscriber> tracing_subscriber::Layer<S> for Collector {
    fn on_event(&self, event: &Event<'_>, _: Context<'_, S>) {
        let mut text = String::new();
        event.record(&mut Fields(&mut text));
        let target = event.metadata().target().to_owned();
        CAPTURE.with(|slot| {
            if let Some(events) = slot.borrow_mut().as_mut() {
                events.push((target, text));
            }
        });
    }
}

/// Выполняет `run` и возвращает события, записанные им на этом потоке.
///
/// Subscriber ставится глобальным один раз на бинарь, а не через `with_default`: пока
/// зарегистрирован единственный диспетчер, tracing-core считает интерес callsite по диспетчеру
/// потока, впервые дошедшего до него. Параллельный тест без своего диспетчера кэширует
/// `Interest::never`, и событие пропадает (так же устроен перехват в `crane-http`).
fn capture<R>(run: impl FnOnce() -> R) -> (R, Captured) {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        tracing::subscriber::set_global_default(tracing_subscriber::registry().with(Collector))
            .unwrap();
    });
    CAPTURE.with(|slot| *slot.borrow_mut() = Some(Vec::new()));
    let result = run();
    let events = CAPTURE
        .with(|slot| slot.borrow_mut().take())
        .unwrap_or_default();
    (result, events)
}

/// Токен из строки печати: между «bootstrap token: » и « (valid until».
fn printed_token(text: &str) -> &str {
    let rest = text.split("bootstrap token: ").nth(1).unwrap_or_default();
    rest.split(" (valid until").next().unwrap_or_default()
}

/// Сколько событий не из `crane::bootstrap` содержат токен.
fn leaks(events: &[(String, String)]) -> usize {
    let printed = events.iter().find(|(target, _)| target == BOOTSTRAP_TARGET);
    let token = printed.map_or("", |(_, text)| printed_token(text));
    events
        .iter()
        .filter(|(target, text)| target != BOOTSTRAP_TARGET && text.contains(token))
        .count()
}

#[test]
fn bootstrap_token_is_printed_once_and_reaches_no_other_target() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let ids = Arc::new(SequentialIdGen::default());
    let audit = Arc::new(OcsfLogSink::new(
        clock.clone(),
        ids.clone(),
        crate::ecosystem::ecosystem,
    ));
    let journal = Journal::new(audit, ids);

    // Act
    let (started, seen) = capture(|| {
        run(start_wizard(
            &store,
            clock.as_ref(),
            &journal,
            Source::Generate,
        ))
    });

    // Assert
    started.unwrap();
    let bootstrap = seen.iter().filter(|(t, _)| t == BOOTSTRAP_TARGET).count();
    let audit_events = seen.iter().filter(|(t, _)| t == AUDIT_TARGET).count();
    let token_length =
        printed_token(&seen.iter().find(|(t, _)| t == BOOTSTRAP_TARGET).unwrap().1).len();
    assert_eq!(
        (bootstrap, audit_events, token_length, leaks(&seen)),
        (1, 1, 47, 0)
    );
}
