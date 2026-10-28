use super::version::is_snapshot;
use crane_domain::registry::{NameError, PackageName};

/// Предел байт имени `groupId:artifactId`: ограничивает ключи каталога и пути layout.
const MAX_LEN: usize = 255;
/// Разделитель `groupId` и `artifactId` в имени пакета.
const COORD_SEPARATOR: char = ':';

/// (`groupId`, `artifactId`) разобранного имени; `None` — имя не из `parse_name`.
pub(crate) fn coordinates(name: &PackageName) -> Option<(&str, &str)> {
    name.normalized().split_once(COORD_SEPARATOR)
}

/// `groupId:artifactId`. Регистр значим (Maven различает `Foo` и `foo`), поэтому нормализованная
/// форма равна написанной.
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
    // Недостающий или лишний разделитель — та же ошибка формы, что у npm для «@scope/name»:
    // `groupId` играет роль scope.
    let (group, artifact) = raw
        .split_once(COORD_SEPARATOR)
        .ok_or(NameError::MalformedScope)?;
    check_group(group)?;
    check_artifact(artifact)?;
    Ok(PackageName::from_parts(raw.to_owned(), raw.to_owned()))
}

/// Сегменты через `.`, каждый непустой; точка — только разделитель, потому что в layout
/// `groupId` раскладывается по каталогам.
fn check_group(group: &str) -> Result<(), NameError> {
    if group.is_empty() {
        return Err(NameError::MalformedScope);
    }
    for segment in group.split('.') {
        if segment.is_empty() {
            return Err(NameError::MalformedScope);
        }
        if let Some(bad) = segment
            .chars()
            .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-')))
        {
            return Err(NameError::InvalidChar(bad));
        }
    }
    Ok(())
}

fn check_artifact(artifact: &str) -> Result<(), NameError> {
    if artifact.is_empty() {
        return Err(NameError::MalformedScope);
    }
    if let Some(bad) = artifact
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')))
    {
        return Err(NameError::InvalidChar(bad));
    }
    // Суффикс `-SNAPSHOT` у artifactId сделал бы путь `…/a-SNAPSHOT/maven-metadata.xml`
    // неотличимым от уровня сборок.
    if matches!(artifact, "." | "..") || is_snapshot(artifact) {
        return Err(NameError::Reserved(artifact.to_owned()));
    }
    Ok(())
}
