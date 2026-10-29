#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap - это падение теста"
)]

use std::time::SystemTime;

use crane_domain::onboarding::{Onboarding, TokenHash};
use crane_memory::MemoryAudit;

use super::*;
use crate::onboarding::tests::{NOW, clock, facts, journal, run, store};

const ALICE: &str = "cli:alice";

fn alice() -> Principal {
    operator(Some("alice"))
}

fn pending(valid_until: SystemTime) -> Onboarding {
    Onboarding::Pending {
        token: TokenHash::of(b"crb_old"),
        expires_at: valid_until,
    }
}

fn completed() -> Onboarding {
    Onboarding::Completed {
        at: NOW,
        by: "crane:system".to_owned(),
    }
}

#[test]
fn rotate_from_pending_replaces_the_token_and_records_the_operator() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let audit = Arc::new(MemoryAudit::default());
    run(store.begin(pending(NOW + TOKEN_TTL))).unwrap();

    // Act
    let issued = run(rotate_token(
        &store,
        clock.as_ref(),
        &journal(&audit),
        &alice(),
    ))
    .unwrap();

    // Assert
    let state = run(store.get()).unwrap().unwrap();
    assert_eq!(
        (
            state.accepts(issued.token.expose().as_bytes(), NOW),
            state.accepts(b"crb_old", NOW),
            facts(&audit)
        ),
        (
            true,
            false,
            vec![(ALICE.to_owned(), AuditAction::OnboardingTokenRotated)]
        )
    );
}

#[test]
fn rotate_from_completed_points_to_reset_and_writes_nothing() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let audit = Arc::new(MemoryAudit::default());
    run(store.begin(completed())).unwrap();

    // Act
    let result = run(rotate_token(
        &store,
        clock.as_ref(),
        &journal(&audit),
        &alice(),
    ));

    // Assert
    assert_eq!(
        (result.err().map(|e| e.to_string()), facts(&audit)),
        (
            Some("мастер завершён, используйте onboarding-reset --confirm".to_owned()),
            vec![]
        )
    );
}

#[test]
fn rotate_and_reset_before_the_first_start_say_it_has_not_begun() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let audit = Arc::new(MemoryAudit::default());
    let confirmed = Confirmed::require(true).unwrap();

    // Act
    let messages = run(async {
        let rotated = rotate_token(&store, clock.as_ref(), &journal(&audit), &alice()).await;
        let reset = reset(
            &store,
            clock.as_ref(),
            &journal(&audit),
            &alice(),
            confirmed,
        )
        .await;
        [rotated.err(), reset.err()].map(|e| e.map(|e| e.to_string()))
    });

    // Assert
    let expected = Some("первый запуск ещё не начат".to_owned());
    assert_eq!(messages, [expected.clone(), expected]);
}

#[test]
fn reset_requires_the_confirm_flag() {
    // Arrange
    let flags = [false, true];

    // Act
    let results = flags.map(|flag| {
        Confirmed::require(flag)
            .map(|_| ())
            .map_err(|e| e.to_string())
    });

    // Assert
    assert_eq!(
        results,
        [
            Err(
                "onboarding-reset открывает мастер заново и аннулирует прежний токен; \
                 повторите с флагом --confirm"
                    .to_owned()
            ),
            Ok(())
        ]
    );
}

#[test]
fn reset_from_completed_reopens_the_wizard_with_a_new_token_and_records_it() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let audit = Arc::new(MemoryAudit::default());
    run(store.begin(completed())).unwrap();
    let confirmed = Confirmed::require(true).unwrap();

    // Act
    let issued = run(reset(
        &store,
        clock.as_ref(),
        &journal(&audit),
        &alice(),
        confirmed,
    ))
    .unwrap();

    // Assert
    let state = run(store.get()).unwrap().unwrap();
    assert_eq!(
        (
            state.accepts(issued.token.expose().as_bytes(), NOW),
            facts(&audit)
        ),
        (true, vec![(ALICE.to_owned(), AuditAction::OnboardingReset)])
    );
}

#[test]
fn operator_is_a_human_cli_principal_with_unknown_fallback() {
    // Arrange
    let users = [Some("alice"), Some(" bob "), Some("  "), None];

    // Act
    let operators = users.map(|user| {
        let principal = operator(user);
        (principal.id, principal.kind)
    });

    // Assert
    let human = PrincipalKind::Human;
    assert_eq!(
        operators,
        [
            ("cli:alice".to_owned(), human),
            ("cli:bob".to_owned(), human),
            ("cli:unknown".to_owned(), human),
            ("cli:unknown".to_owned(), human),
        ]
    );
}
