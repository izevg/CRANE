use crane_domain::registry::{Version, VersionError};

use super::generic::{CLASS_BUILD, items, key_prefix, terminate};

/// Предел байт версии: версия — сегмент пути и часть ключа порядка.
const MAX_LEN: usize = 128;
/// Символы, запрещённые в версии моделью Maven (`ILLEGAL_VERSION_CHARS`), плюс `%` и `#`:
/// путь layout приходит декодированным, и `%2e` в сегменте разбирается как попытка обхода.
const FORBIDDEN: &str = "\\/:\"<>|?*%#";
const SNAPSHOT_SUFFIX: &str = "-SNAPSHOT";
/// `yyyyMMdd` и `HHmmss` в имени timestamp-сборки.
const DATE_DIGITS: usize = 8;
const TIME_DIGITS: usize = 6;

/// Timestamp-сборка SNAPSHOT: `1.0-20261028.123456-3` -> база `1.0`, представление `1.0-SNAPSHOT`.
///
/// Единственное место правила: по нему версия получает ключ порядка, путь файла — каталог
/// `X-SNAPSHOT`, а ретенция и metadata — группировку сборок.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Build<'a> {
    /// База без суффикса `-SNAPSHOT`.
    pub base: &'a str,
    /// `yyyyMMdd` числом.
    pub date: u32,
    /// `HHmmss` числом: ведущие нули восстанавливает [`Build::timestamp`].
    pub time: u32,
    pub number: u32,
}

impl<'a> Build<'a> {
    /// Разбирает `X-yyyyMMdd.HHmmss-N`; всё прочее (включая `X-SNAPSHOT`) — не сборка.
    #[must_use]
    pub fn parse(version: &'a str) -> Option<Self> {
        let (rest, number) = version.rsplit_once('-')?;
        let (base, stamp) = rest.rsplit_once('-')?;
        let (date, time) = stamp.split_once('.')?;
        if base.is_empty() {
            return None;
        }
        Some(Self {
            base,
            date: fixed_digits(date, DATE_DIGITS)?,
            time: fixed_digits(time, TIME_DIGITS)?,
            number: digits(number)?,
        })
    }

    /// Каталог и представление всех сборок базы: `1.0` -> `1.0-SNAPSHOT`.
    #[must_use]
    pub fn base_version(&self) -> String {
        format!("{}{SNAPSHOT_SUFFIX}", self.base)
    }

    /// Порядок сборок внутри базы: сначала время, потом номер.
    #[must_use]
    pub const fn sequence(&self) -> (u32, u32, u32) {
        (self.date, self.time, self.number)
    }

    /// `yyyyMMdd.HHmmss`, как в `<snapshot><timestamp>` и в имени файла.
    #[must_use]
    pub fn timestamp(&self) -> String {
        format!("{:08}.{:06}", self.date, self.time)
    }
}

/// Десятичное число из ASCII-цифр. `str::parse` принял бы и ведущий `+`, поэтому цифры
/// проверяются отдельно; пустая строка и переполнение `u32` дают `None`.
fn digits(text: &str) -> Option<u32> {
    if text.bytes().all(|b| b.is_ascii_digit()) {
        text.parse().ok()
    } else {
        None
    }
}

fn fixed_digits(text: &str, len: usize) -> Option<u32> {
    if text.len() == len {
        digits(text)
    } else {
        None
    }
}

/// Версия Maven: каноническая форма равна написанной (литеральный каталог layout), порядок —
/// `GenericVersionScheme`, а `prerelease` значит «timestamp-сборка SNAPSHOT».
pub(crate) fn parse_version(raw: &str) -> Result<Version, VersionError> {
    check_grammar(raw)?;
    let (key, prerelease) = match Build::parse(raw) {
        // Сборка стоит под релизом `X` и упорядочена внутри базы по (время, номер).
        Some(build) => (
            key_prefix(&items(&build.base_version()))
                .rank(CLASS_BUILD)
                .number(build.date.into())
                .number(build.time.into())
                .number(build.number.into())
                .finish(),
            true,
        ),
        None => (terminate(key_prefix(&items(raw))).finish(), false),
    };
    Ok(Version::from_parts(
        raw.to_owned(),
        raw.to_owned(),
        key,
        prerelease,
    ))
}

fn check_grammar(raw: &str) -> Result<(), VersionError> {
    let invalid = |reason: &str| VersionError::Invalid {
        raw: raw.to_owned(),
        reason: reason.to_owned(),
    };
    if raw.is_empty() {
        return Err(invalid("версия пуста"));
    }
    if raw.len() > MAX_LEN {
        return Err(invalid("версия длиннее допустимого"));
    }
    if matches!(raw, "." | "..") {
        return Err(invalid("версия не может быть «.» или «..»"));
    }
    // Версия — каталог layout: только видимый ASCII без символов, запрещённых моделью Maven.
    if let Some(bad) = raw
        .chars()
        .find(|c| !c.is_ascii_graphic() || FORBIDDEN.contains(*c))
    {
        return Err(invalid(&format!("недопустимый символ {bad:?}")));
    }
    Ok(())
}
