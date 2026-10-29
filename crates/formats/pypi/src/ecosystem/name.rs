use crane_domain::registry::{NameError, PackageName};

/// Знаки, которые PEP 503 сворачивает в один дефис.
const SEPARATORS: [char; 3] = ['-', '_', '.'];

/// Имя проекта по core metadata (PEP 508): `^([A-Z0-9]|[A-Z0-9][A-Z0-9._-]*[A-Z0-9])$`
/// без учёта регистра, только ASCII. Написание сохраняется как прислано, нормализуется по PEP 503.
pub(crate) fn parse_name(raw: &str) -> Result<PackageName, NameError> {
    let (Some(first), Some(last)) = (raw.chars().next(), raw.chars().next_back()) else {
        return Err(NameError::Empty);
    };
    if let Some(bad) = raw
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || SEPARATORS.contains(c)))
    {
        return Err(NameError::InvalidChar(bad));
    }
    if !first.is_ascii_alphanumeric() {
        return Err(NameError::InvalidStart(first));
    }
    // Отдельного варианта «недопустимый конец» в domain нет: граница тоже недопустимый символ.
    if !last.is_ascii_alphanumeric() {
        return Err(NameError::InvalidChar(last));
    }
    Ok(PackageName::from_parts(normalize(raw), raw.to_owned()))
}

/// PEP 503: нижний регистр, любая серия `-`, `_`, `.` — один дефис.
pub(crate) fn normalize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_run = false;
    for c in raw.to_ascii_lowercase().chars() {
        if SEPARATORS.contains(&c) {
            if !in_run {
                out.push('-');
            }
            in_run = true;
        } else {
            out.push(c);
            in_run = false;
        }
    }
    out
}
