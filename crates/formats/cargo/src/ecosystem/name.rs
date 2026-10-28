use crane_domain::registry::{NameError, PackageName};

const MAX_LEN: usize = 64;

/// Правила crates.io; зарезервированные имена — политика, а не грамматика.
pub(crate) fn parse_name(raw: &str) -> Result<PackageName, NameError> {
    let Some(first) = raw.chars().next() else {
        return Err(NameError::Empty);
    };
    let len = raw.chars().count();
    if len > MAX_LEN {
        return Err(NameError::TooLong { len, max: MAX_LEN });
    }
    if !first.is_ascii_alphabetic() {
        return Err(NameError::InvalidStart(first));
    }
    if let Some(bad) = raw
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
    {
        return Err(NameError::InvalidChar(bad));
    }
    Ok(PackageName::from_parts(normalize(raw), raw.to_owned()))
}

/// Как `canon_crate_name` в crates.io: `-` и `_` обозначают одно имя.
pub(super) fn normalize(raw: &str) -> String {
    raw.to_ascii_lowercase().replace('-', "_")
}
