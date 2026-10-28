//! Имя загружаемого файла: безопасность пути и разбор имён wheel и sdist.

use super::UploadError;
use crate::ecosystem::parse_version;

const WHEEL_SUFFIX: &str = ".whl";
const SDIST_SUFFIX: &str = ".tar.gz";
/// Предел длины имени файла большинства файловых систем.
const MAX_LEN: usize = 255;
const WHEEL_PARTS: [usize; 2] = [5, 6];

/// Вид дистрибутива; другие (`.zip`-sdist, `.egg`, `.exe`) `PyPI` больше не принимает.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Distribution {
    Wheel,
    Sdist,
}

impl Distribution {
    /// Значение поля `filetype` формы загрузки.
    pub(super) fn from_filetype(filetype: &str) -> Result<Self, UploadError> {
        match filetype {
            "bdist_wheel" => Ok(Self::Wheel),
            "sdist" => Ok(Self::Sdist),
            other => Err(UploadError::Filetype(other.to_owned())),
        }
    }
}

/// Что имя файла говорит о проекте и версии.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct FileName<'a> {
    pub(super) distribution: Distribution,
    pub(super) project: &'a str,
    pub(super) version: &'a str,
}

/// Имя станет путём файла в хранилище и сегментом URL, поэтому допустим один сегмент из узкого
/// набора ASCII: без разделителей пути, управляющих символов, `%`, `#`, `?` и пробелов.
pub(super) fn check_path(filename: &str) -> Result<(), UploadError> {
    if filename.is_empty() {
        return Err(UploadError::EmptyFilename);
    }
    let safe_chars = filename
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '+' | '!'));
    if !safe_chars || filename.len() > MAX_LEN || matches!(filename, "." | "..") {
        return Err(UploadError::UnsafeFilename);
    }
    Ok(())
}

pub(super) fn parse(filename: &str) -> Result<FileName<'_>, UploadError> {
    if let Some(stem) = filename.strip_suffix(WHEEL_SUFFIX) {
        wheel(stem)
    } else if let Some(stem) = filename.strip_suffix(SDIST_SUFFIX) {
        sdist(stem)
    } else {
        Err(UploadError::UnsupportedFile(filename.to_owned()))
    }
}

/// `{dist}-{version}(-{build})?-{python}-{abi}-{platform}`: остальные компоненты дефиса не
/// содержат, поэтому делим по `-` и ждём 5 или 6 частей. Build tag начинается с цифры.
fn wheel(stem: &str) -> Result<FileName<'_>, UploadError> {
    let parts: Vec<&str> = stem.split('-').collect();
    let build_ok = parts.len() != 6 || parts[2].starts_with(|c: char| c.is_ascii_digit());
    if !WHEEL_PARTS.contains(&parts.len()) || parts.contains(&"") || !build_ok {
        return Err(UploadError::MalformedFilename("wheel"));
    }
    Ok(FileName {
        distribution: Distribution::Wheel,
        project: parts[0],
        version: parts[1],
    })
}

/// `{name}-{version}` (PEP 625). Версия может содержать дефис только в легаси-написании, поэтому
/// делим по последнему дефису, справа от которого стоит валидная версия.
fn sdist(stem: &str) -> Result<FileName<'_>, UploadError> {
    stem.rmatch_indices('-')
        .map(|(at, _)| (&stem[..at], &stem[at + 1..]))
        .find(|(_, version)| parse_version(version).is_ok())
        .map(|(project, version)| FileName {
            distribution: Distribution::Sdist,
            project,
            version,
        })
        .ok_or(UploadError::MalformedFilename("sdist"))
}
