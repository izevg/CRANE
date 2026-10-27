//! Факты для журнала аудита.

use crate::access::{Principal, RequestContext};
use crate::registry::Coordinate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditAction {
    Authenticate,
    Publish(Coordinate),
    Yank(Coordinate),
    Unyank(Coordinate),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Success,
    Failure { reason: String },
}

/// Факт для журнала. `event_id` и время ставит приёмник.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// `None` только у неуспешной `Authenticate`: субъект не установлен.
    pub actor: Option<Principal>,
    pub request: RequestContext,
    pub action: AuditAction,
    pub outcome: Outcome,
}
