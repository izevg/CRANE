//! Факты для журнала аудита.

use crate::access::{Principal, RequestContext};
use crate::registry::{Coordinate, Format, Namespace, PackageName, PackageRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditAction {
    Authenticate,
    Publish(Coordinate),
    Yank(Coordinate),
    Unyank(Coordinate),
    Delete(Coordinate),
    Deprecate {
        coordinate: Coordinate,
        /// `None` снимает устаревание.
        message: Option<String>,
    },
    Tag {
        coordinate: Coordinate,
        tag: String,
    },
    Untag {
        package: PackageRef,
        tag: String,
    },
    /// Резервирование имён: конфигурационный namespace или первая публикация точного имени.
    Reserve(Namespace),
    /// Прокси отказался отдавать внутреннее имя из публичного upstream.
    ShadowUpstream {
        format: Format,
        package: PackageName,
    },
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
