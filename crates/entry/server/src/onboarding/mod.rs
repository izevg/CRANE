//! Первый запуск в composition root: какая фаза у процесса, выпуск bootstrap-токена и роли
//! `onboarding-token`, `onboarding-reset` (разделы 20.2 и 20.5).

mod bootstrap;
mod roles;
mod token;

use std::sync::Arc;
use std::time::Duration;

use crane_domain::access::{Principal, RequestContext, RequestId};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::onboarding::Onboarding;
use crane_ports::audit::AuditSink;
use crane_ports::settings::{OnboardingError, OnboardingStore};
use crane_ports::system::{Clock, IdGen};

pub(crate) use bootstrap::{BOOTSTRAP_TARGET, Source, announce, reissue_expired};
pub(crate) use roles::{onboarding_reset, onboarding_token};
use token::RandomError;

/// Как часто фаза мастера смотрит в строку `onboarding`: на любой реплике одинаково.
pub(crate) const WIZARD_POLL: Duration = Duration::from_secs(5);

/// Строка `onboarding` глазами выбора фазы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Row {
    Missing,
    Pending,
    Completed,
}

impl Row {
    pub(crate) const fn of(state: Option<&Onboarding>) -> Self {
        match state {
            None => Self::Missing,
            Some(Onboarding::Pending { .. }) => Self::Pending,
            Some(Onboarding::Completed { .. }) => Self::Completed,
        }
    }
}

/// Что делает очередной проход цикла фаз.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Step {
    /// Строки нет, разбор проходит: `begin(Completed)` и рабочий режим.
    Preconfigure,
    /// Строки нет, разбор не проходит: `begin(Pending)` с токеном.
    StartWizard,
    Wizard,
    Ready,
    /// Процесс не поднимается: ошибка разбора называет ключ и причину.
    Fail,
}

/// Фазу задаёт состояние `onboarding`, а результат разбора лишь разрешает рабочий режим.
/// При `memory` строки нет и мастера нет: разбор решает всё, как в v0.2 (записать мастеру некуда).
pub(crate) const fn decide(row: Row, parsed_ok: bool, memory: bool) -> Step {
    match (memory, row, parsed_ok) {
        (true, _, true) | (false, Row::Completed, true) => Step::Ready,
        (true, _, false) | (false, Row::Completed, false) => Step::Fail,
        (false, Row::Missing, true) => Step::Preconfigure,
        (false, Row::Missing, false) => Step::StartWizard,
        (false, Row::Pending, _) => Step::Wizard,
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum OnboardingFailure {
    #[error(transparent)]
    Random(#[from] RandomError),
    #[error(transparent)]
    Store(#[from] OnboardingError),
}

/// Приёмник событий аудита с идентификаторами запросов: у процесса запроса нет, у события есть.
pub(crate) struct Journal {
    sink: Arc<dyn AuditSink>,
    ids: Arc<dyn IdGen>,
}

impl Journal {
    pub(crate) fn new(sink: Arc<dyn AuditSink>, ids: Arc<dyn IdGen>) -> Self {
        Self { sink, ids }
    }

    pub(crate) fn record(&self, actor: &Principal, action: AuditAction) {
        self.sink.record(AuditEvent {
            actor: Some(actor.clone()),
            request: RequestContext {
                request_id: RequestId::from(self.ids.generate()),
                trace_id: None,
                client_ip: None,
                user_agent: None,
            },
            action,
            outcome: Outcome::Success,
        });
    }
}

/// «Предконфигурирована»: до этого запуска строки не было, а конфигурация разбирается (v0.1-v0.2).
pub(crate) async fn preconfigure<S: OnboardingStore>(
    store: &S,
    clock: &dyn Clock,
    journal: &Journal,
) -> Result<(), OnboardingFailure> {
    let system = Principal::system();
    let state = store
        .begin(Onboarding::Completed {
            at: clock.now(),
            by: system.id.clone(),
        })
        .await?;
    // Если другая реплика успела открыть мастер (`Pending`), события нет. Две предконфигурированные
    // реплики запишут по событию: оба факта верны, а победителя у `Completed` нет.
    if matches!(state, Onboarding::Completed { ref by, .. } if *by == system.id) {
        journal.record(&system, AuditAction::OnboardingSkippedPreconfigured);
    }
    Ok(())
}

/// Первый старт без конфигурации: `begin(Pending)`, печать токена победителем и событие `OnboardingStarted`.
pub(crate) async fn start_wizard<S: OnboardingStore>(
    store: &S,
    clock: &dyn Clock,
    journal: &Journal,
    source: Source,
) -> Result<(), OnboardingFailure> {
    let begun = bootstrap::begin(store, clock, source.mint()?).await?;
    if begun.won {
        journal.record(&Principal::system(), AuditAction::OnboardingStarted);
    }
    if let Some(issued) = &begun.announce {
        announce(issued);
    }
    Ok(())
}

/// Перевыпуск истёкшего токена на входе в фазу мастера.
pub(crate) async fn refresh_token<S: OnboardingStore>(
    store: &S,
    clock: &dyn Clock,
    journal: &Journal,
    state: Option<&Onboarding>,
    source: Source,
) -> Result<(), OnboardingFailure> {
    let Some(state) = state else { return Ok(()) };
    if let Some(reissued) = reissue_expired(store, clock, state, source).await? {
        journal.record(&Principal::system(), AuditAction::OnboardingTokenRotated);
        if let Some(issued) = &reissued.announce {
            announce(issued);
        }
    }
    Ok(())
}

/// Завершается, когда мастер завершён. Ошибка чтения не гасит фазу: БД моргнула, опрос продолжается.
pub(crate) async fn until_completed<S: OnboardingStore>(store: Arc<S>, every: Duration) {
    loop {
        match store.get().await {
            Ok(Some(Onboarding::Completed { .. })) => return,
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "не удалось прочитать состояние первого запуска"),
        }
        tokio::time::sleep(every).await;
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
