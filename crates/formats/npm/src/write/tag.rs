//! dist-tags: правила имён и тела запросов.

use std::collections::BTreeMap;

use crate::ecosystem::parse_version;
use crane_domain::registry::Version;

use super::WriteError;

const TAG_MAX_LEN: usize = 128;
const LATEST: &str = "latest";

/// Имя тега: непустое, ≤ 128, URL-безопасное и не версия (правило npm: иначе `pkg@1.0.0` неоднозначно).
pub fn parse_tag(raw: &str) -> Result<String, WriteError> {
    if raw.is_empty() {
        return Err(WriteError::TagEmpty);
    }
    if raw.len() > TAG_MAX_LEN {
        return Err(WriteError::TagTooLong);
    }
    if let Some(bad) = raw
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || "-._~".contains(*c)))
    {
        return Err(WriteError::TagChar(bad));
    }
    if parse_version(raw).is_ok() {
        return Err(WriteError::TagIsVersion);
    }
    Ok(raw.to_owned())
}

/// `latest` снять нельзя (как на npmjs): при удалении версий он выводится при рендере.
pub fn removable(tag: &str) -> Result<(), WriteError> {
    if tag == LATEST {
        return Err(WriteError::LatestRemoval);
    }
    Ok(())
}

/// Тело `PUT dist-tags/{tag}`: JSON-строка с версией.
pub fn parse_tag_body(body: &[u8]) -> Result<Version, WriteError> {
    let raw: String = serde_json::from_slice(body)?;
    parse_version(&raw).map_err(WriteError::Version)
}

/// Ответ `GET dist-tags`: `{"tag":"version"}` в детерминированном порядке.
#[must_use]
pub fn render_tags<'a>(tags: impl IntoIterator<Item = (&'a str, &'a Version)>) -> String {
    let map: BTreeMap<&str, &str> = tags
        .into_iter()
        .map(|(tag, version)| (tag, version.as_written()))
        .collect();
    crate::reply::to_json(&map)
}
