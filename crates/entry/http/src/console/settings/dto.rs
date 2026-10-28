//! `SettingView` → схема `Setting` из common.yaml. Значения секрета в представлении нет по построению.

use std::time::SystemTime;

use crane_app::settings::{SettingView, Shown, StoredShown};
use crane_domain::settings::{Kind, Origin, Unit, Unreadable};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::SETTINGS_HREF;

pub(super) fn setting(view: &SettingView) -> Value {
    let def = &view.def;
    let (secret_state, unreadable) = secret_facts(view);
    json!({
        "key": def.key,
        "env_var": def.env_var,
        "group": def.group,
        "kind": kind_name(def.kind),
        "unit": def.unit.map(unit_name),
        "origin": origin_name(view.origin()),
        "locked": view.locked(),
        "value": value(view),
        "secret_state": secret_state,
        "unreadable": unreadable,
        "version": view.stored.as_ref().map(|row| row.version.get()),
        "updated_at": view.stored.as_ref().map(|row| timestamp(row.updated_at)),
        "updated_by": view.stored.as_ref().map(|row| row.updated_by.as_str()),
        "shadowed": view.shadowed().map(shadowed),
        "_links": {"self": {"href": format!("{SETTINGS_HREF}/{}", def.key)}},
    })
}

/// Значение действующего слоя. Секрет и незаданный ключ — `null`: умолчания схема не хранит.
fn value(view: &SettingView) -> Option<&str> {
    if view.def.kind == Kind::Secret {
        return None;
    }
    match (view.origin(), &view.stored) {
        (Origin::Env, _) => view
            .inherited
            .as_ref()
            .and_then(|inherited| inherited.value.as_deref()),
        (
            Origin::Stored,
            Some(StoredShown {
                value: Shown::Plain(text),
                ..
            }),
        ) => Some(text),
        _ => None,
    }
}

/// Состояние секрета по действующему слою: окружение перекрывает нечитаемую строку.
fn secret_facts(view: &SettingView) -> (Option<&'static str>, Option<Value>) {
    if view.def.kind != Kind::Secret {
        return (None, None);
    }
    match (view.origin(), view.stored.as_ref().map(|row| &row.value)) {
        (Origin::Stored, Some(Shown::Unreadable(reason))) => {
            (Some("unreadable"), Some(unreadable(reason)))
        }
        (Origin::Env | Origin::Stored, _) => (Some("set"), None),
        (Origin::Default, _) => (Some("unset"), None),
    }
}

fn unreadable(reason: &Unreadable) -> Value {
    match reason {
        Unreadable::MissingKey => json!({"reason": "missing_key"}),
        Unreadable::WrongKey {
            stored_key_id,
            current_key_id,
        } => json!({
            "reason": "wrong_key",
            "stored_key_id": stored_key_id,
            "current_key_id": current_key_id,
        }),
        Unreadable::Corrupted => json!({"reason": "corrupted"}),
    }
}

fn shadowed(row: &StoredShown) -> Value {
    json!({
        "version": row.version.get(),
        "updated_at": timestamp(row.updated_at),
        "updated_by": row.updated_by,
    })
}

fn timestamp(at: SystemTime) -> String {
    // Формат RFC 3339 для допустимых дат не отказывает; пустая строка лучше паники в обработчике.
    OffsetDateTime::from(at)
        .format(&Rfc3339)
        .unwrap_or_default()
}

const fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Text => "text",
        Kind::Number => "number",
        Kind::Bool => "bool",
        Kind::List => "list",
        Kind::Url => "url",
        Kind::Secret => "secret",
    }
}

const fn unit_name(unit: Unit) -> &'static str {
    match unit {
        Unit::Bytes => "bytes",
        Unit::Seconds => "seconds",
    }
}

pub(super) const fn origin_name(origin: Origin) -> &'static str {
    match origin {
        Origin::Env => "env",
        Origin::Stored => "stored",
        Origin::Default => "default",
    }
}
