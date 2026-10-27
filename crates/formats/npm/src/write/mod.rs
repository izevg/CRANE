//! Протокол записи: `PUT` документа никогда ничего не разрушает, а классифицируется в одну из команд.
//!
//! Снятие версии или пакета — только явные `DELETE` (см. `route`): npm CLI шлёт их после PUT,
//! а файл в пути однозначно называет версию. Поэтому устаревший документ не может снять чужую публикацию.

mod deprecate;
mod publish;
mod tag;

use std::borrow::Cow;
use std::collections::BTreeMap;

use crane_domain::registry::{NameError, PackageVersion, Version, VersionError};
use serde::Deserialize;

use crate::packument::{Fields, Raw};

pub use publish::{NpmPublish, valid_for_new};
pub use tag::{parse_tag, parse_tag_body, removable, render_tags};

/// Команда, в которую превратился `PUT`.
#[derive(Debug, PartialEq, Eq)]
pub enum Write {
    Publish(NpmPublish),
    /// Версия и новое сообщение (`None` — снять устаревание); только реальные изменения.
    Deprecate(Vec<(Version, Option<String>)>),
    /// Например, правка maintainers: не поддерживается.
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum WriteError {
    #[error("некорректный JSON: {0}")]
    Json(String),
    #[error("публикация должна содержать ровно одну версию")]
    NotSingleVersion,
    #[error("публикация должна содержать ровно одно вложение")]
    NotSingleAttachment,
    #[error("имя в документе не совпадает с пакетом запроса")]
    NameMismatch,
    #[error("версия в манифесте не совпадает с ключом версии")]
    VersionMismatch,
    #[error("имя вложения не соответствует имени tarball версии")]
    AttachmentMismatch,
    #[error("вложение не является корректным base64")]
    Base64,
    #[error("заявленная длина вложения {declared} не совпадает с фактической {actual}")]
    LengthMismatch { declared: u64, actual: u64 },
    #[error("в dist нет integrity")]
    MissingIntegrity,
    #[error("dist.integrity содержит некорректную запись")]
    BadIntegrity,
    #[error("в dist.integrity нет sha512")]
    MissingSha512,
    #[error("dist.shasum не является sha1 в hex")]
    BadShasum,
    #[error("публикация должна нести ровно один dist-tag, указывающий на публикуемую версию")]
    BadDistTags,
    #[error("в имени нового пакета недопустим символ {0:?}: только строчные буквы, цифры и «-._»")]
    NewNameChar(char),
    #[error("зависимость {name:?} указывает вне реестра: {spec:?}")]
    ForeignDependency { name: String, spec: String },
    #[error("поле deprecated должно быть строкой")]
    DeprecatedNotString,
    #[error("некорректная версия")]
    Version(#[source] VersionError),
    #[error("некорректное имя пакета")]
    Name(#[source] NameError),
    #[error("имя тега пусто")]
    TagEmpty,
    #[error("имя тега длиннее 128 символов")]
    TagTooLong,
    #[error("недопустимый символ {0:?} в имени тега")]
    TagChar(char),
    #[error("имя тега не может быть версией")]
    TagIsVersion,
    #[error("тег latest нельзя удалить")]
    LatestRemoval,
}

impl From<serde_json::Error> for WriteError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error.to_string())
    }
}

/// Читаемая часть тела `PUT`; остальное игнорируется.
#[derive(Deserialize)]
struct Body<'a> {
    #[serde(borrow)]
    name: Option<Cow<'a, str>>,
    #[serde(borrow, rename = "_attachments")]
    attachments: Option<BTreeMap<Cow<'a, str>, Attachment<'a>>>,
    #[serde(borrow)]
    versions: Option<BTreeMap<Cow<'a, str>, Fields<'a>>>,
    #[serde(borrow, rename = "dist-tags")]
    dist_tags: Option<BTreeMap<Cow<'a, str>, Cow<'a, str>>>,
}

#[derive(Deserialize)]
struct Attachment<'a> {
    #[serde(borrow)]
    data: Cow<'a, str>,
    length: Option<u64>,
}

/// Строковое поле манифеста.
fn string_field(fields: &Fields<'_>, key: &str) -> Option<String> {
    fields.get(key).and_then(Raw::as_string)
}

/// Определяет, что значит `PUT` пакета; `current` — версии, уже лежащие в реестре.
pub fn classify(
    package: &crane_domain::registry::PackageName,
    body: &[u8],
    current: &[PackageVersion],
) -> Result<Write, WriteError> {
    let parsed: Body<'_> = serde_json::from_slice(body)?;
    if parsed.attachments.is_some() {
        return publish::publish(package, parsed).map(Write::Publish);
    }
    match parsed.versions {
        Some(versions) => deprecate::deprecations(versions, current).map(Write::Deprecate),
        None => Ok(Write::Unsupported),
    }
}
