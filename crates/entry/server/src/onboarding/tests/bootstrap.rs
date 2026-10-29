#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap - это падение теста"
)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crane_domain::onboarding::TokenHash;
use crane_domain::settings::{Secret, Stored, StoredValue, Unreadable, Version};
use crane_memory::{ManualClock, MemoryOnboarding};

use super::*;
use crate::onboarding::tests::{NOW, clock, run, store};

const PROVIDED: &str = "crb_0123456789012345678901234567890123456789";

fn env(pairs: &[(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> + use<> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |name| map.get(name).cloned()
}

fn no_files(_: &Path) -> io::Result<String> {
    Err(io::Error::from(io::ErrorKind::NotFound))
}

fn provided() -> Source {
    Source::Provided(BootstrapToken::provided(Secret::new(PROVIDED.to_owned())))
}

fn label(source: &Result<Source, ConfigError>) -> String {
    match source {
        Ok(Source::Static) => "static".to_owned(),
        Ok(Source::Generate) => "generate".to_owned(),
        Ok(Source::Provided(token)) => format!("provided:{}", token.expose()),
        Err(error) => error.to_string(),
    }
}

#[test]
fn source_prefers_static_token_then_provided_then_generates() {
    // Arrange
    let cases = [
        vec![("CRANE_TOKEN", "any")],
        vec![("CRANE_TOKEN_FILE", "/run/secrets/token")],
        vec![("CRANE_TOKEN", "any"), ("CRANE_BOOTSTRAP_TOKEN", PROVIDED)],
        vec![("CRANE_BOOTSTRAP_TOKEN", PROVIDED)],
        vec![],
    ];

    // Act
    let sources = cases.map(|pairs| label(&Source::resolve(&env(&pairs), &no_files, false)));

    // Assert
    assert_eq!(
        sources,
        [
            "static".to_owned(),
            "static".to_owned(),
            "static".to_owned(),
            format!("provided:{PROVIDED}"),
            "generate".to_owned(),
        ]
    );
}

#[test]
fn provided_token_without_prefix_or_too_short_is_rejected_without_echoing_it() {
    // Arrange
    let cases = [
        vec![(
            "CRANE_BOOTSTRAP_TOKEN",
            "no-prefix-0123456789012345678901234567890123",
        )],
        vec![("CRANE_BOOTSTRAP_TOKEN", "crb_short")],
    ];

    // Act
    let errors = cases.map(|pairs| label(&Source::resolve(&env(&pairs), &no_files, false)));

    // Assert
    let expected = "некорректное значение CRANE_BOOTSTRAP_TOKEN: \
                    ожидался токен вида crb_… не короче 36 символов";
    assert_eq!(errors, [expected, expected]);
}

#[test]
fn begin_lets_only_the_race_winner_announce_the_token() {
    // Arrange
    let clock = clock();
    let store = store(&clock);

    // Act
    let outcomes = run(async {
        let first = begin(&store, clock.as_ref(), Source::Generate.mint().unwrap()).await;
        let second = begin(&store, clock.as_ref(), Source::Generate.mint().unwrap()).await;
        (first.unwrap(), second.unwrap())
    });

    // Assert
    let (first, second) = outcomes;
    let accepted = first
        .announce
        .as_ref()
        .map(|issued| (issued.valid_until, first.won));
    assert_eq!(
        (accepted, second.won, second.announce.is_some()),
        (Some((NOW + TOKEN_TTL, true)), false, false)
    );
}

#[test]
fn announced_token_is_the_one_stored_and_loser_token_is_not() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let winner = Source::Generate.mint().unwrap();
    let loser = Source::Generate.mint().unwrap();
    let (winner_text, loser_text) = (
        winner.token.expose().to_owned(),
        loser.token.expose().to_owned(),
    );

    // Act
    let state = run(async {
        begin(&store, clock.as_ref(), winner).await.unwrap();
        begin(&store, clock.as_ref(), loser).await.unwrap();
        store.get().await.unwrap().unwrap()
    });

    // Assert
    assert_eq!(
        (
            state.accepts(winner_text.as_bytes(), NOW),
            state.accepts(loser_text.as_bytes(), NOW)
        ),
        (true, false)
    );
}

#[test]
fn begin_with_provided_token_stores_it_and_prints_nothing() {
    // Arrange
    let clock = clock();
    let store = store(&clock);

    // Act
    let begun = run(async {
        let begun = begin(&store, clock.as_ref(), provided().mint().unwrap())
            .await
            .unwrap();
        (begun, store.get().await.unwrap().unwrap())
    });

    // Assert
    let (begun, state) = begun;
    assert_eq!(
        (
            begun.won,
            begun.announce.is_some(),
            state.accepts(PROVIDED.as_bytes(), NOW)
        ),
        (true, false, true)
    );
}

#[test]
fn begin_with_static_token_opens_pending_but_nothing_is_announced_or_accepted() {
    // Arrange
    let clock = clock();
    let store = store(&clock);

    // Act
    let result = run(async {
        let begun = begin(&store, clock.as_ref(), Source::Static.mint().unwrap())
            .await
            .unwrap();
        (begun, store.get().await.unwrap().unwrap())
    });

    // Assert
    let (begun, state) = result;
    assert_eq!(
        (
            begun.won,
            begun.announce.is_some(),
            matches!(state, Onboarding::Pending { .. }),
            state.accepts(b"static-token", NOW)
        ),
        (true, false, true, false)
    );
}

fn pending_for_a_day(clock: &Arc<ManualClock>) -> (MemoryOnboarding, Onboarding) {
    let store = store(clock);
    let initial = Onboarding::Pending {
        token: TokenHash::of(b"crb_old"),
        expires_at: NOW + TOKEN_TTL,
    };
    let state = run(store.begin(initial)).unwrap();
    (store, state)
}

#[test]
fn expired_token_is_reissued_and_announced_from_the_boundary_second() {
    // Arrange
    let clock = clock();
    let (store, state) = pending_for_a_day(&clock);
    clock.advance(TOKEN_TTL);

    // Act
    let reissued = run(reissue_expired(
        &store,
        clock.as_ref(),
        &state,
        Source::Generate,
    ));

    // Assert
    let announced = reissued.unwrap().and_then(|r| r.announce).unwrap();
    let stored = run(store.get()).unwrap().unwrap();
    assert_eq!(
        (
            stored.accepts(announced.token.expose().as_bytes(), NOW + TOKEN_TTL),
            stored.accepts(b"crb_old", NOW + TOKEN_TTL),
            announced.valid_until
        ),
        (true, false, NOW + TOKEN_TTL * 2)
    );
}

#[test]
fn unexpired_token_survives_restart_and_is_not_announced_again() {
    // Arrange
    let clock = clock();
    let (store, state) = pending_for_a_day(&clock);
    clock.advance(TOKEN_TTL.checked_sub(Duration::from_secs(1)).unwrap());

    // Act
    let reissued = run(reissue_expired(
        &store,
        clock.as_ref(),
        &state,
        Source::Generate,
    ));

    // Assert
    let stored = run(store.get()).unwrap().unwrap();
    assert_eq!(
        (reissued.unwrap().is_some(), stored.accepts(b"crb_old", NOW)),
        (false, true)
    );
}

#[test]
fn expired_token_with_static_token_is_left_alone() {
    // Arrange
    let clock = clock();
    let (store, state) = pending_for_a_day(&clock);
    clock.advance(TOKEN_TTL * 2);

    // Act
    let reissued = run(reissue_expired(
        &store,
        clock.as_ref(),
        &state,
        Source::Static,
    ));

    // Assert
    let stored = run(store.get()).unwrap().unwrap();
    assert_eq!(
        (reissued.unwrap().is_some(), stored.accepts(b"crb_old", NOW)),
        (false, true)
    );
}

#[test]
fn expired_token_with_provided_one_is_renewed_quietly() {
    // Arrange
    let clock = clock();
    let (store, state) = pending_for_a_day(&clock);
    clock.advance(TOKEN_TTL * 2);
    let later = NOW + TOKEN_TTL * 2;

    // Act
    let reissued = run(reissue_expired(&store, clock.as_ref(), &state, provided()));

    // Assert
    let stored = run(store.get()).unwrap().unwrap();
    assert_eq!(
        (
            reissued.unwrap().map(|r| r.announce.is_some()),
            stored.accepts(PROVIDED.as_bytes(), later)
        ),
        (Some(false), true)
    );
}

#[test]
fn completed_wizard_is_never_reissued() {
    // Arrange
    let clock = clock();
    let store = store(&clock);
    let completed = Onboarding::Completed {
        at: NOW,
        by: "crane:system".to_owned(),
    };
    let state = run(store.begin(completed)).unwrap();
    clock.advance(TOKEN_TTL * 2);

    // Act
    let reissued = run(reissue_expired(
        &store,
        clock.as_ref(),
        &state,
        Source::Generate,
    ));

    // Assert
    assert!(reissued.unwrap().is_none());
}

#[test]
fn utc_formats_rfc_3339_across_leap_days_and_year_ends() {
    // Arrange
    let seconds = [0, 951_782_400, 1_709_164_800, 1_767_225_599, 1_793_102_400];

    // Act
    let formatted = seconds.map(|s| utc(SystemTime::UNIX_EPOCH + Duration::from_secs(s)));

    // Assert
    assert_eq!(
        formatted,
        [
            "1970-01-01T00:00:00Z",
            "2000-02-29T00:00:00Z",
            "2024-02-29T00:00:00Z",
            "2025-12-31T23:59:59Z",
            "2026-10-27T12:00:00Z",
        ]
    );
}

fn row(value: StoredValue) -> Stored {
    Stored {
        value,
        version: Version::FIRST,
        updated_at: SystemTime::UNIX_EPOCH,
        updated_by: "admin".to_owned(),
    }
}

#[test]
fn stored_token_counts_as_static_only_when_readable() {
    // Arrange
    let secret = row(StoredValue::Secret(Secret::new("t".to_owned())));
    let unreadable = row(StoredValue::Unreadable(Unreadable::MissingKey));
    let other = row(StoredValue::Plain("x".to_owned()));
    let cases = [
        BTreeMap::from([("token".to_owned(), secret)]),
        BTreeMap::from([("token".to_owned(), unreadable)]),
        BTreeMap::from([("token_principal".to_owned(), other)]),
        BTreeMap::new(),
    ];

    // Act
    let set = cases.each_ref().map(stored_token_set);

    // Assert
    assert_eq!(set, [true, false, false, false]);
}

#[test]
fn stored_token_makes_the_source_static_so_expired_bootstrap_is_not_reissued() {
    // Arrange
    let clock = clock();
    let (store, state) = pending_for_a_day(&clock);
    clock.advance(TOKEN_TTL * 2);
    let source = Source::resolve(&env(&[]), &no_files, true).unwrap();

    // Act
    let reissued = run(reissue_expired(&store, clock.as_ref(), &state, source));

    // Assert
    let stored = run(store.get()).unwrap().unwrap();
    assert_eq!(
        (reissued.unwrap().is_some(), stored.accepts(b"crb_old", NOW)),
        (false, true)
    );
}
