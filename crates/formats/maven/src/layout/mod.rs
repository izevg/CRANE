//! Раскладка Maven-репозитория: `g/a/v/a-v[-c].ext`, `g/a/maven-metadata.xml`,
//! `g/a/X-SNAPSHOT/maven-metadata.xml` и суффиксы контрольных сумм.
//!
//! Разбор разделён на три уровня. [`Route::parse`] знает только структуру пути и безопасность
//! сегментов, поэтому proxy не зависит от грамматики версий. Смысл имени файла даёт
//! [`FileName::parse`], версию — `Ecosystem::parse_version`.

mod checksum;
mod file;

use crane_domain::proxy::{PathError, UpstreamPath};
use crane_domain::registry::{Ecosystem, NameError, PackageName};

pub use checksum::Algorithm;
pub use file::FileName;

use crate::ecosystem::{MAVEN, SNAPSHOT_SUFFIX, coordinates, snapshot_base};

const METADATA_FILE: &str = "maven-metadata.xml";
/// Кодированные `.`, `/` и `\`: путь приходит декодированным, поэтому в сегменте они
/// означают попытку обхода, которую поймает следующий слой.
const ENCODED_TRAVERSAL: [&str; 3] = ["%2e", "%2f", "%5c"];

/// Разобранный путь layout. Метода нет: набор ресурсов одинаков для GET, HEAD и PUT,
/// действие выбирает `entry` по паре (метод, ресурс).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Route {
    pub resource: Resource,
    /// Путь заканчивался суффиксом суммы: ресурс тот же, представление — сумма.
    pub checksum: Option<Algorithm>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resource {
    /// `g/a/<dir>/<file>`: `dir` — сегмент каталога (релиз или `X-SNAPSHOT`), а не версия;
    /// `file` — имя без суффикса суммы.
    File {
        package: PackageName,
        dir: String,
        file: String,
    },
    /// `g/a/maven-metadata.xml`. Групповой уровень плагинов (`g/maven-metadata.xml`) по
    /// синтаксису неотличим и разбирается так же: известное ограничение.
    Versions { package: PackageName },
    /// `g/a/X-SNAPSHOT/maven-metadata.xml`; `base` — `X` без суффикса.
    Builds { package: PackageName, base: String },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RouteError {
    #[error("небезопасный сегмент пути")]
    UnsafePath,
    #[error("путь не соответствует раскладке Maven")]
    NotLayout,
    #[error("некорректное имя артефакта")]
    Name(#[source] NameError),
    #[error("имя файла не соответствует артефакту и версии каталога")]
    FileMismatch,
    #[error("в каталоге SNAPSHOT нужна уникальная сборка `X-yyyyMMdd.HHmmss-N`")]
    NotUniqueSnapshot,
    #[error("timestamp-сборка допустима только в каталоге своей базы `X-SNAPSHOT`")]
    BuildOutsideSnapshot,
}

impl Route {
    /// `path` — декодированный остаток после `/maven/<repo>/`.
    pub fn parse(path: &str) -> Result<Self, RouteError> {
        let segments: Vec<&str> = path.split('/').collect();
        segments
            .iter()
            .try_for_each(|segment| check_segment(segment))?;
        let (last, rest) = segments.split_last().ok_or(RouteError::NotLayout)?;
        let (name, checksum) = split_checksum(last);
        if name.is_empty() {
            return Err(RouteError::NotLayout);
        }
        let resource = if name == METADATA_FILE {
            metadata_resource(rest)?
        } else {
            file_resource(rest, name)?
        };
        Ok(Self { resource, checksum })
    }

    /// Путь у upstream строится из разобранного, а не берётся из URL.
    pub fn upstream_path(&self) -> Result<UpstreamPath, PathError> {
        let suffix = self
            .checksum
            .map_or_else(String::new, |alg| format!(".{}", alg.suffix()));
        let path = match &self.resource {
            Resource::File { package, dir, file } => {
                format!("{}/{dir}/{file}{suffix}", package_dir(package)?)
            }
            Resource::Versions { package } => {
                format!("{}/{METADATA_FILE}{suffix}", package_dir(package)?)
            }
            Resource::Builds { package, base } => format!(
                "{}/{base}{SNAPSHOT_SUFFIX}/{METADATA_FILE}{suffix}",
                package_dir(package)?
            ),
        };
        UpstreamPath::parse(&path)
    }
}

/// Сегмент пути: непустой, не `.`/`..`, без `\`, управляющих символов и кодированных `. / \`.
fn check_segment(segment: &str) -> Result<(), RouteError> {
    let lower = segment.to_ascii_lowercase();
    let unsafe_segment = segment.is_empty()
        || matches!(segment, "." | "..")
        || segment.contains('\\')
        || segment.chars().any(char::is_control)
        || ENCODED_TRAVERSAL
            .iter()
            .any(|encoded| lower.contains(encoded));
    if unsafe_segment {
        Err(RouteError::UnsafePath)
    } else {
        Ok(())
    }
}

/// Имя без суффикса суммы и сам алгоритм.
fn split_checksum(name: &str) -> (&str, Option<Algorithm>) {
    name.rsplit_once('.')
        .and_then(|(stem, suffix)| Algorithm::from_suffix(suffix).map(|alg| (stem, Some(alg))))
        .unwrap_or((name, None))
}

/// `…/maven-metadata.xml`: каталог `X-SNAPSHOT` выше — уровень сборок, иначе уровень артефакта.
fn metadata_resource(dirs: &[&str]) -> Result<Resource, RouteError> {
    match dirs.split_last() {
        Some((dir, package_dirs)) => match snapshot_base(dir) {
            Some(base) => Ok(Resource::Builds {
                package: package(package_dirs)?,
                base: base.to_owned(),
            }),
            None => Ok(Resource::Versions {
                package: package(dirs)?,
            }),
        },
        None => Err(RouteError::NotLayout),
    }
}

/// `g/a/<dir>/<file>`.
fn file_resource(dirs: &[&str], file: &str) -> Result<Resource, RouteError> {
    let (dir, package_dirs) = dirs.split_last().ok_or(RouteError::NotLayout)?;
    Ok(Resource::File {
        package: package(package_dirs)?,
        dir: (*dir).to_owned(),
        file: file.to_owned(),
    })
}

/// Каталоги `g/…/a` -> имя `g.…:a`. Точка внутри сегмента группы отвергается: иначе у одного
/// пакета было бы несколько адресов.
fn package(dirs: &[&str]) -> Result<PackageName, RouteError> {
    let (artifact, groups) = dirs.split_last().ok_or(RouteError::NotLayout)?;
    if groups.is_empty() || groups.iter().any(|group| group.contains('.')) {
        return Err(RouteError::NotLayout);
    }
    MAVEN
        .parse_name(&format!("{}:{artifact}", groups.join(".")))
        .map_err(RouteError::Name)
}

/// `g.h:a` -> `g/h/a`.
fn package_dir(package: &PackageName) -> Result<String, PathError> {
    // Имя не из `parse_name` не имеет координат, а значит и пути.
    let (group, artifact) = coordinates(package).ok_or(PathError::Empty)?;
    Ok(format!("{}/{artifact}", group.replace('.', "/")))
}

#[cfg(test)]
mod tests;
