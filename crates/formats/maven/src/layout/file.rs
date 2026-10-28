use super::RouteError;
use crate::ecosystem::Build;

const SNAPSHOT_SUFFIX: &str = "-SNAPSHOT";
const SNAPSHOT_WORD: &str = "SNAPSHOT";
/// Длина `yyyyMMdd.HHmmss`.
const STAMP_LEN: usize = 15;

/// Смысл имени файла в каталоге версии: `{artifact}-{version}[-{classifier}].{extension}`.
///
/// Classifier и extension нигде не хранятся: их всегда выводят из пути asset этим же разбором,
/// которым путь был принят при записи.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileName<'a> {
    /// Версия каталога; для `X-SNAPSHOT` это timestamp-сборка `X-yyyyMMdd.HHmmss-N` из имени.
    pub version: &'a str,
    pub classifier: Option<&'a str>,
    /// Остаток после первой точки: `jar`, `pom`, `module`, `tar.gz`, `jar.asc`.
    pub extension: &'a str,
}

impl<'a> FileName<'a> {
    /// Разбирает имя файла `file` в каталоге `dir` артефакта `artifact`.
    ///
    /// В каталоге `X-SNAPSHOT` принимаются только уникальные сборки с базой `X`. Имя
    /// `a-1.0-SNAPSHOT.jar` отвергается: оно перезаписываемо, а версия в CRANE неизменна.
    pub fn parse(artifact: &str, dir: &str, file: &'a str) -> Result<Self, RouteError> {
        let rest = file
            .strip_prefix(artifact)
            .and_then(|rest| rest.strip_prefix('-'))
            .ok_or(RouteError::FileMismatch)?;
        let version_len = match dir.strip_suffix(SNAPSHOT_SUFFIX) {
            Some(base) => build_len(base, rest)?,
            None => release_len(dir, rest)?,
        };
        let (version, tail) = rest.split_at(version_len);
        let (classifier, extension) = split_tail(tail)?;
        Ok(Self {
            version,
            classifier,
            extension,
        })
    }
}

/// Длина версии-сборки в начале `rest`: `{base}-{yyyyMMdd.HHmmss}-{N}`.
fn build_len(base: &str, rest: &str) -> Result<usize, RouteError> {
    let after_base = rest
        .strip_prefix(base)
        .and_then(|after| after.strip_prefix('-'))
        .ok_or(RouteError::FileMismatch)?;
    if after_base.starts_with(SNAPSHOT_WORD) {
        return Err(RouteError::NotUniqueSnapshot);
    }
    // Номер сборки — цифры после отметки времени и дефиса; дальше `-classifier` или `.ext`.
    let number_at = base.len() + 1 + STAMP_LEN + 1;
    let number_len = rest.get(number_at..).map_or(0, |number| {
        number.bytes().take_while(u8::is_ascii_digit).count()
    });
    let version = rest
        .get(..number_at + number_len)
        .ok_or(RouteError::FileMismatch)?;
    // Длины фиксированы, а в отметке времени и номере нет дефисов: база разобранной сборки
    // совпадает с `base` по построению, отдельной сверки не нужно.
    Build::parse(version)
        .map(|_| version.len())
        .ok_or(RouteError::FileMismatch)
}

/// Длина версии релиза: имя файла несёт каталог буквально.
fn release_len(dir: &str, rest: &str) -> Result<usize, RouteError> {
    if Build::parse(dir).is_some() {
        // Сборка живёт только в каталоге своей базы `X-SNAPSHOT`.
        return Err(RouteError::BuildOutsideSnapshot);
    }
    rest.starts_with(dir)
        .then_some(dir.len())
        .ok_or(RouteError::FileMismatch)
}

/// `-classifier.ext` или `.ext`.
fn split_tail(tail: &str) -> Result<(Option<&str>, &str), RouteError> {
    let (classifier, extension) = match tail.strip_prefix('-') {
        Some(with_classifier) => {
            let (classifier, extension) = with_classifier
                .split_once('.')
                .ok_or(RouteError::FileMismatch)?;
            if classifier.is_empty() || !classifier.chars().all(is_name_char) {
                return Err(RouteError::FileMismatch);
            }
            (Some(classifier), extension)
        }
        None => (
            None,
            tail.strip_prefix('.').ok_or(RouteError::FileMismatch)?,
        ),
    };
    let valid = extension
        .split('.')
        .all(|part| !part.is_empty() && part.chars().all(is_name_char));
    if valid {
        Ok((classifier, extension))
    } else {
        Err(RouteError::FileMismatch)
    }
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '-')
}
