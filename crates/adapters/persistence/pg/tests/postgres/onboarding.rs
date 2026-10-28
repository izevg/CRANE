//! Мастер первого запуска: гонка реплик при `begin`.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crane_domain::onboarding::{Onboarding, TokenHash};
use crane_ports::settings::{OnboardingError, OnboardingStore};

use crate::CONTENDERS;
use crate::support;
use crate::support::settings::clock;

const NOW: SystemTime = SystemTime::UNIX_EPOCH;

fn pending(token: &str) -> Onboarding {
    Onboarding::Pending {
        token: TokenHash::of(token.as_bytes()),
        expires_at: NOW + Duration::from_secs(3_600),
    }
}

#[test]
fn concurrent_begin_returns_the_same_winner_to_every_replica() {
    support::run(async |server| {
        // Arrange
        let pool = server.fresh_pool().await;
        let tokens: Vec<String> = (0..CONTENDERS).map(|n| format!("crb_{n}")).collect();
        let tasks = tokens
            .iter()
            .map(|token| {
                let store = Arc::new(crane_pg::PgOnboarding::new(pool.clone(), clock()));
                let initial = pending(token);
                tokio::spawn(async move { store.begin(initial).await })
            })
            .collect();

        // Act
        let results = support::join_all(tasks).await;

        // Assert
        let seen: Vec<Vec<bool>> = results
            .iter()
            .map(|result| {
                let state = result.as_ref().expect("begin отвечает состоянием");
                tokens
                    .iter()
                    .map(|token| state.accepts(token.as_bytes(), NOW))
                    .collect()
            })
            .collect();
        let winners = seen[0].iter().filter(|accepted| **accepted).count();
        assert!(seen.iter().all(|view| *view == seen[0]));
        assert_eq!(winners, 1);
    });
}

#[test]
fn closed_pool_makes_onboarding_store_unreachable() {
    support::run(async |server| {
        // Arrange
        let pool = server.fresh_pool().await;
        let store = crane_pg::PgOnboarding::new(pool.clone(), clock());
        pool.close().await;

        // Act
        let result = store.get().await;

        // Assert
        assert!(matches!(result, Err(OnboardingError::Unreachable(_))));
    });
}
