use crane_domain::registry::{NameError, PackageName};

/// Предел npm в байтах всего имени, включая `@scope/`.
const MAX_LEN: usize = 214;
/// Имена, которые npm держит за собой (`validate-npm-package-name`).
const RESERVED: [&str; 2] = ["node_modules", "favicon.ico"];

/// Грамматика npm, допустимая для того, что может отдавать upstream (в том числе старые имена
/// с заглавными буквами вроде `JSONStream`). Строже для новых пакетов — политика формата, не грамматика.
/// Регистр сохраняется и в нормализованной форме: имена npm регистрозависимы.
pub(crate) fn parse_name(raw: &str) -> Result<PackageName, NameError> {
    if raw.is_empty() {
        return Err(NameError::Empty);
    }
    if raw.len() > MAX_LEN {
        return Err(NameError::TooLong {
            len: raw.len(),
            max: MAX_LEN,
        });
    }
    if RESERVED.contains(&raw) {
        return Err(NameError::Reserved(raw.to_owned()));
    }
    match raw.strip_prefix('@') {
        Some(scoped) => {
            let (scope, name) = scoped.split_once('/').ok_or(NameError::MalformedScope)?;
            if scope.is_empty() || name.is_empty() {
                return Err(NameError::MalformedScope);
            }
            check_part(scope)?;
            check_part(name)?;
        }
        None => check_part(raw)?,
    }
    Ok(PackageName::from_parts(raw.to_owned(), raw.to_owned()))
}

/// Часть имени npm (scope или имя): не пустая, не с `.`/`_`, только URL-безопасные символы.
fn check_part(part: &str) -> Result<(), NameError> {
    if let Some(first) = part.chars().next().filter(|c| matches!(c, '.' | '_')) {
        return Err(NameError::InvalidStart(first));
    }
    match part
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || "-._~!'()*".contains(*c)))
    {
        Some(bad) => Err(NameError::InvalidChar(bad)),
        None => Ok(()),
    }
}
