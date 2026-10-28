use std::time::SystemTime;

use crane_domain::access::Principal;
use crane_domain::onboarding::{Onboarding, TokenHash};

#[derive(Debug, thiserror::Error)]
pub enum OnboardingError {
    /// Мастер завершён навсегда; открыть его заново может только `reset`.
    #[error("первый запуск уже завершён")]
    Completed,
    #[error("первый запуск ещё не начат")]
    NotStarted,
    #[error("ошибка хранилища первого запуска")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Единственная строка состояния мастера.
pub trait OnboardingStore: Send + Sync {
    fn get(&self) -> impl Future<Output = Result<Option<Onboarding>, OnboardingError>> + Send;

    /// Вставляет `initial`, если строки нет, и возвращает то, что лежит в хранилище:
    /// при гонке реплик побеждает одна, остальные получают её состояние.
    fn begin(
        &self,
        initial: Onboarding,
    ) -> impl Future<Output = Result<Onboarding, OnboardingError>> + Send;

    /// Новый токен вместо истёкшего или потерянного. Только из `Pending`.
    fn rotate(
        &self,
        token: TokenHash,
        expires_at: SystemTime,
    ) -> impl Future<Output = Result<(), OnboardingError>> + Send;

    /// Break-glass: из любого существующего состояния обратно в `Pending`.
    fn reset(
        &self,
        token: TokenHash,
        expires_at: SystemTime,
    ) -> impl Future<Output = Result<(), OnboardingError>> + Send;

    /// Только из `Pending`; токен стирается. Время завершения ставит хранилище (как `updated_at`).
    fn complete(&self, by: &Principal) -> impl Future<Output = Result<(), OnboardingError>> + Send;
}
