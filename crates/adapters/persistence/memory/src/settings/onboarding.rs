use std::fmt;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::SystemTime;

use crane_domain::access::Principal;
use crane_domain::onboarding::{Onboarding, TokenHash};
use crane_ports::settings::{OnboardingError, OnboardingStore};
use crane_ports::system::Clock;

/// Состояние мастера в памяти: одна строка под замком. Время завершения берёт из `Clock`.
pub struct MemoryOnboarding {
    clock: Arc<dyn Clock>,
    state: Mutex<Option<Onboarding>>,
}

impl fmt::Debug for MemoryOnboarding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Внутри хэш токена, поэтому содержимое не печатается.
        f.debug_struct("MemoryOnboarding").finish_non_exhaustive()
    }
}

impl MemoryOnboarding {
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            state: Mutex::new(None),
        }
    }

    // Замок берётся только в синхронных функциях: guard не должен жить через await.
    fn lock(&self) -> MutexGuard<'_, Option<Onboarding>> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn begin_sync(&self, initial: Onboarding) -> Onboarding {
        self.lock().get_or_insert(initial).clone()
    }

    fn rotate_sync(&self, token: TokenHash, expires_at: SystemTime) -> Result<(), OnboardingError> {
        let mut state = self.lock();
        match state.as_ref() {
            None => Err(OnboardingError::NotStarted),
            Some(Onboarding::Completed { .. }) => Err(OnboardingError::Completed),
            Some(Onboarding::Pending { .. }) => {
                *state = Some(Onboarding::Pending { token, expires_at });
                Ok(())
            }
        }
    }

    fn reset_sync(&self, token: TokenHash, expires_at: SystemTime) -> Result<(), OnboardingError> {
        let mut state = self.lock();
        if state.is_none() {
            return Err(OnboardingError::NotStarted);
        }
        *state = Some(Onboarding::Pending { token, expires_at });
        Ok(())
    }

    fn complete_sync(&self, by: &Principal) -> Result<(), OnboardingError> {
        let mut state = self.lock();
        match state.as_ref() {
            None => Err(OnboardingError::NotStarted),
            Some(Onboarding::Completed { .. }) => Err(OnboardingError::Completed),
            Some(Onboarding::Pending { .. }) => {
                *state = Some(Onboarding::Completed {
                    at: self.clock.now(),
                    by: by.id.clone(),
                });
                Ok(())
            }
        }
    }
}

impl OnboardingStore for MemoryOnboarding {
    fn get(&self) -> impl Future<Output = Result<Option<Onboarding>, OnboardingError>> + Send {
        std::future::ready(Ok(self.lock().clone()))
    }

    fn begin(
        &self,
        initial: Onboarding,
    ) -> impl Future<Output = Result<Onboarding, OnboardingError>> + Send {
        std::future::ready(Ok(self.begin_sync(initial)))
    }

    fn rotate(
        &self,
        token: TokenHash,
        expires_at: SystemTime,
    ) -> impl Future<Output = Result<(), OnboardingError>> + Send {
        std::future::ready(self.rotate_sync(token, expires_at))
    }

    fn reset(
        &self,
        token: TokenHash,
        expires_at: SystemTime,
    ) -> impl Future<Output = Result<(), OnboardingError>> + Send {
        std::future::ready(self.reset_sync(token, expires_at))
    }

    fn complete(&self, by: &Principal) -> impl Future<Output = Result<(), OnboardingError>> + Send {
        std::future::ready(self.complete_sync(by))
    }
}
