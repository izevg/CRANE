//! Роли бинарника `onboarding-token` и `onboarding-reset`: break-glass только с хоста, без HTTP.

use std::sync::Arc;

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::audit::AuditAction;
use crane_pg::PgOnboarding;
use crane_ports::settings::{OnboardingError, OnboardingStore};
use crane_ports::system::Clock;
use crane_siem::OcsfLogSink;
use tokio::runtime::Builder;

use super::Journal;
use super::bootstrap::{Issued, TOKEN_TTL, announce};
use super::token::{BootstrapToken, RandomError};
use crate::config::Database;
use crate::ecosystem::ecosystem;
use crate::run::{RunError, connect};
use crate::system::{SystemClock, UuidV7Ids};

#[derive(Debug, thiserror::Error)]
pub(crate) enum RoleError {
    #[error("мастер завершён, используйте onboarding-reset --confirm")]
    Completed,
    #[error("первый запуск ещё не начат")]
    NotStarted,
    #[error(
        "onboarding-reset открывает мастер заново и аннулирует прежний токен; \
         повторите с флагом --confirm"
    )]
    ConfirmationRequired,
    #[error("роль требует PostgreSQL: при CRANE_DATABASE_URL=memory первого запуска нет")]
    NoDatabase,
    #[error(transparent)]
    Random(#[from] RandomError),
    #[error(transparent)]
    Store(OnboardingError),
}

impl From<OnboardingError> for RoleError {
    fn from(error: OnboardingError) -> Self {
        match error {
            OnboardingError::Completed => Self::Completed,
            OnboardingError::NotStarted => Self::NotStarted,
            other => Self::Store(other),
        }
    }
}

/// Доказательство того, что оператор передал `--confirm`: сброс без него не построить.
pub(crate) struct Confirmed(());

impl Confirmed {
    pub(crate) const fn require(flag: bool) -> Result<Self, RoleError> {
        if flag {
            Ok(Self(()))
        } else {
            Err(RoleError::ConfirmationRequired)
        }
    }
}

/// Оператор CLI в журнале: `cli:<USER>`; без переменной `cli:unknown`.
pub(crate) fn operator(user: Option<&str>) -> Principal {
    let name = user.map(str::trim).filter(|name| !name.is_empty());
    Principal {
        id: format!("cli:{}", name.unwrap_or("unknown")),
        kind: PrincipalKind::Human,
    }
}

/// Новый токен вместо потерянного или истёкшего; прежний аннулируется. Только из `Pending`.
pub(crate) async fn rotate_token<S: OnboardingStore>(
    store: &S,
    clock: &dyn Clock,
    journal: &Journal,
    operator: &Principal,
) -> Result<Issued, RoleError> {
    let token = BootstrapToken::generate()?;
    let valid_until = clock.now() + TOKEN_TTL;
    store.rotate(token.hash(), valid_until).await?;
    journal.record(operator, AuditAction::OnboardingTokenRotated);
    Ok(Issued { token, valid_until })
}

/// Break-glass: из любого состояния обратно в `Pending` с новым токеном; сделанные настройки остаются.
pub(crate) async fn reset<S: OnboardingStore>(
    store: &S,
    clock: &dyn Clock,
    journal: &Journal,
    operator: &Principal,
    _: Confirmed,
) -> Result<Issued, RoleError> {
    let token = BootstrapToken::generate()?;
    let valid_until = clock.now() + TOKEN_TTL;
    store.reset(token.hash(), valid_until).await?;
    journal.record(operator, AuditAction::OnboardingReset);
    Ok(Issued { token, valid_until })
}

pub(crate) fn onboarding_token(database: Database) -> Result<(), RunError> {
    with_store(database, |store, clock, journal, operator| async move {
        rotate_token(&store, clock.as_ref(), &journal, &operator).await
    })
}

pub(crate) fn onboarding_reset(database: Database, confirmed: bool) -> Result<(), RunError> {
    // Отказ до подключения к БД: подсказка важнее ошибки сети.
    let confirmation = Confirmed::require(confirmed)?;
    with_store(database, |store, clock, journal, operator| async move {
        reset(&store, clock.as_ref(), &journal, &operator, confirmation).await
    })
}

/// Подключение, выполнение операции и печать выданного токена. Только `Database`: полный `Config` не нужен.
fn with_store<F, Fut>(database: Database, operation: F) -> Result<(), RunError>
where
    F: FnOnce(PgOnboarding, Arc<dyn Clock>, Journal, Principal) -> Fut,
    Fut: Future<Output = Result<Issued, RoleError>>,
{
    let Database::Postgres(options) = database else {
        return Err(RoleError::NoDatabase.into());
    };
    let runtime = Builder::new_current_thread().enable_all().build()?;
    runtime.block_on(async {
        let pool = connect(&options).await?;
        drop(options);
        let clock: Arc<dyn Clock> = Arc::new(SystemClock);
        let ids = Arc::new(UuidV7Ids);
        let journal = Journal::new(
            Arc::new(OcsfLogSink::new(Arc::clone(&clock), ids.clone(), ecosystem)),
            ids,
        );
        let store = PgOnboarding::new(pool, Arc::clone(&clock));
        let operator = operator(std::env::var("USER").ok().as_deref());
        let issued = operation(store, clock, journal, operator).await?;
        announce(&issued);
        Ok(())
    })
}

#[cfg(test)]
#[path = "tests/roles.rs"]
mod tests;
