//! Use cases настроек: просмотр, запись и удаление с аудитом каждого исхода.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

#[path = "../common/mod.rs"]
mod common;
mod delete;
mod fixtures;
mod put;
mod view;

use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_memory::MemoryAudit;

use crate::common::{principal, request_context};

fn event(action: AuditAction, outcome: Outcome) -> AuditEvent {
    AuditEvent {
        actor: Some(principal()),
        request: request_context(),
        action,
        outcome,
    }
}

/// Действие и код отказа каждого события: сравнивается без текста причины.
fn actions_with_codes(audit: &MemoryAudit) -> Vec<(AuditAction, Option<&'static str>)> {
    audit
        .events()
        .into_iter()
        .map(|event| {
            let code = match event.outcome {
                Outcome::Failure { code, .. } => code,
                Outcome::Success => None,
            };
            (event.action, code)
        })
        .collect()
}

fn rejected(key: &str, env_var: Option<&str>) -> AuditAction {
    AuditAction::SettingRejected {
        key: key.to_owned(),
        env_var: env_var.map(str::to_owned),
    }
}
