//! Чистое сопоставление `AuditEvent` с плоскими полями OCSF 1.3.

use crane_domain::access::{Principal, PrincipalKind};
use crane_domain::audit::{AuditAction, AuditEvent, Outcome};
use crane_domain::registry::{Format, PackageName, Version};
use uuid::Uuid;

const SCHEMA_VERSION: &str = "1.3.0";
const PRODUCT_NAME: &str = "CRANE";
const CLASS_AUTHENTICATION: u32 = 3002;
const CLASS_API_ACTIVITY: u32 = 6003;
const CATEGORY_IAM: u32 = 3;
const CATEGORY_APPLICATION: u32 = 6;
const USER_TYPE_UNKNOWN: u32 = 0;
const USER_TYPE_USER: u32 = 1;
const USER_TYPE_OTHER: u32 = 99;

/// Субъект события: `actor.user.*` у 6003, `user.*` у 3002.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcsfUser {
    pub uid: Option<String>,
    pub type_id: u32,
    pub type_name: String,
}

/// Плоские поля одной записи. Необязательные значения — `None`: в журнал они не попадают.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OcsfRecord {
    pub class_uid: u32,
    pub category_uid: u32,
    pub activity_id: u32,
    pub activity_name: &'static str,
    pub type_uid: u32,
    pub status_id: u32,
    pub status: &'static str,
    pub status_detail: Option<String>,
    pub severity_id: u32,
    pub event_id: Uuid,
    pub correlation_uid: String,
    pub schema_version: &'static str,
    pub product_name: &'static str,
    pub product_version: &'static str,
    pub time_ms: u64,
    pub actor: Option<OcsfUser>,
    pub user: Option<OcsfUser>,
    pub src_ip: Option<String>,
    pub user_agent: Option<String>,
    pub operation: Option<&'static str>,
    pub resource_uid: Option<String>,
    pub resource_type: Option<&'static str>,
    pub repository: Option<String>,
    pub trace_id: Option<String>,
}

/// Класс, категория, activity (id и имя) и операция API по действию.
struct Shape {
    class_uid: u32,
    category_uid: u32,
    activity_id: u32,
    activity_name: &'static str,
    operation: Option<&'static str>,
}

fn shape(action: &AuditAction) -> Shape {
    let api = |activity_id, activity_name, operation| Shape {
        class_uid: CLASS_API_ACTIVITY,
        category_uid: CATEGORY_APPLICATION,
        activity_id,
        activity_name,
        operation: Some(operation),
    };
    match action {
        AuditAction::Authenticate => Shape {
            class_uid: CLASS_AUTHENTICATION,
            category_uid: CATEGORY_IAM,
            activity_id: 1,
            activity_name: "Logon",
            operation: None,
        },
        AuditAction::Publish(_) => api(1, "Create", "publish"),
        AuditAction::Yank(_) => api(3, "Update", "yank"),
        AuditAction::Unyank(_) => api(3, "Update", "unyank"),
        AuditAction::Delete(_) => api(4, "Delete", "delete"),
        AuditAction::Deprecate { message, .. } => api(
            3,
            "Update",
            if message.is_some() {
                "deprecate"
            } else {
                "undeprecate"
            },
        ),
        AuditAction::Tag { .. } => api(3, "Update", "tag"),
        AuditAction::Untag { .. } => api(3, "Update", "untag"),
        AuditAction::Reserve(_) => api(1, "Create", "namespace.reserve"),
        AuditAction::ShadowUpstream { .. } => api(2, "Read", "upstream.shadow"),
    }
}

/// Объект действия: пакет и, если действие над версией, сама версия.
struct Resource<'a> {
    format: Format,
    repository: &'a str,
    package: &'a PackageName,
    version: Option<&'a Version>,
}

fn resource(action: &AuditAction) -> Option<Resource<'_>> {
    match action {
        AuditAction::Authenticate
        | AuditAction::Reserve(_)
        | AuditAction::ShadowUpstream { .. } => None,
        AuditAction::Publish(c)
        | AuditAction::Yank(c)
        | AuditAction::Unyank(c)
        | AuditAction::Delete(c)
        | AuditAction::Deprecate { coordinate: c, .. }
        | AuditAction::Tag { coordinate: c, .. } => Some(Resource {
            format: c.format,
            repository: &c.repository,
            package: &c.package,
            version: Some(&c.version),
        }),
        AuditAction::Untag { package, .. } => Some(Resource {
            format: package.format,
            repository: &package.repository,
            package: &package.package,
            version: None,
        }),
    }
}

/// `+` (метаданные сборки) и `@` (scope npm) кодируются: иначе purl неоднозначен.
fn purl_component(raw: &str) -> String {
    raw.replace('+', "%2B").replace('@', "%40")
}

fn purl(r: &Resource<'_>) -> String {
    let name = purl_component(r.package.as_written());
    let base = format!("pkg:{}/{name}", r.format.as_str());
    r.version.map_or(base.clone(), |version| {
        format!("{base}@{}", purl_component(version.as_written()))
    })
}

fn kind_name(kind: PrincipalKind) -> &'static str {
    match kind {
        PrincipalKind::Human => "User",
        PrincipalKind::Robot => "Robot",
        PrincipalKind::Workload => "Workload",
        PrincipalKind::Agent => "Agent",
    }
}

fn user(principal: Option<&Principal>) -> OcsfUser {
    match principal {
        None => OcsfUser {
            uid: None,
            type_id: USER_TYPE_UNKNOWN,
            type_name: "Unknown".to_owned(),
        },
        Some(p) => OcsfUser {
            uid: Some(p.id.clone()),
            type_id: if p.kind == PrincipalKind::Human {
                USER_TYPE_USER
            } else {
                USER_TYPE_OTHER
            },
            type_name: kind_name(p.kind).to_owned(),
        },
    }
}

/// `event_id` и время ставит приёмник.
#[must_use]
pub fn ocsf(event: &AuditEvent, event_id: Uuid, time_ms: u64) -> OcsfRecord {
    let shape = shape(&event.action);
    let subject = user(event.actor.as_ref());
    let is_auth = shape.class_uid == CLASS_AUTHENTICATION;
    let (status_id, status, severity_id, status_detail) = match &event.outcome {
        Outcome::Success => (1, "Success", 1, None),
        Outcome::Failure { reason } => (2, "Failure", 2, Some(reason.clone())),
    };
    let resource = resource(&event.action);
    OcsfRecord {
        class_uid: shape.class_uid,
        category_uid: shape.category_uid,
        activity_id: shape.activity_id,
        activity_name: shape.activity_name,
        type_uid: shape.class_uid * 100 + shape.activity_id,
        status_id,
        status,
        status_detail,
        severity_id,
        event_id,
        correlation_uid: event.request.request_id.as_str().to_owned(),
        schema_version: SCHEMA_VERSION,
        product_name: PRODUCT_NAME,
        product_version: env!("CARGO_PKG_VERSION"),
        time_ms,
        actor: (!is_auth).then(|| subject.clone()),
        user: is_auth.then_some(subject),
        src_ip: event.request.client_ip.map(|ip| ip.to_string()),
        user_agent: event.request.user_agent.clone(),
        operation: shape.operation,
        resource_uid: resource.as_ref().map(purl),
        resource_type: resource.as_ref().map(|_| "package"),
        repository: resource.map(|r| r.repository.to_owned()),
        trace_id: event.request.trace_id.clone(),
    }
}

#[cfg(test)]
#[path = "tests/ocsf.rs"]
mod tests;
