//! `PUT` с документом без вложений: меняет только `deprecated` у уже известных версий.

use std::borrow::Cow;
use std::collections::BTreeMap;

use crane_domain::registry::{Format, PackageVersion, Version};

use super::WriteError;
use crate::packument::{Fields, Raw};

pub(super) fn deprecations(
    versions: BTreeMap<Cow<'_, str>, Fields<'_>>,
    current: &[PackageVersion],
) -> Result<Vec<(Version, Option<String>)>, WriteError> {
    let mut changes = Vec::new();
    for (key, fields) in versions {
        let Ok(version) = Version::parse(Format::Npm, &key) else {
            continue;
        };
        let Some(existing) = current.iter().find(|c| c.version == version) else {
            continue;
        };
        let Some(raw) = fields.get("deprecated") else {
            continue;
        };
        let message = match Raw::as_string(raw) {
            Some(text) if text.is_empty() => None,
            Some(text) => Some(text),
            None => return Err(WriteError::DeprecatedNotString),
        };
        if message != existing.deprecated {
            changes.push((version, message));
        }
    }
    Ok(changes)
}
