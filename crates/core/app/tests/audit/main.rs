//! Аудит: каждая мутация даёт ровно одно событие, чтения не пишут в журнал.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

#[path = "../common/mod.rs"]
mod common;
mod delete;
mod deprecate_tag;
mod publish;
mod reads;
mod yank;

use crane_domain::audit::{AuditAction, AuditEvent, Outcome};

use crate::common::{principal, request_context};

fn event(action: AuditAction, outcome: Outcome) -> AuditEvent {
    AuditEvent {
        actor: Some(principal()),
        request: request_context(),
        action,
        outcome,
    }
}

fn failure(reason: &str) -> Outcome {
    Outcome::Failure {
        reason: reason.to_owned(),
        code: None,
    }
}
