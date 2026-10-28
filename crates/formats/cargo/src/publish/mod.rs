//! Разбор тела `PUT /api/v1/crates/new`.

mod metadata;

use std::fmt;

use bytes::Bytes;
use crane_domain::registry::{
    Format, FormatMetadata, NameError, PackageName, Version, VersionError,
};

use metadata::PublishJson;

/// Опубликованная версия, разобранная из тела запроса.
pub struct CargoPublish {
    pub name: PackageName,
    pub version: Version,
    pub crate_file: Bytes,
    // Сырой JSON хранится до применения политики: см. `index_metadata`.
    json: PublishJson,
}

impl CargoPublish {
    /// Метаданные для хранения в индексе.
    ///
    /// `own_index` — адрес индекса хостящего репозитория (`sparse+…/index/`): зависимости на него
    /// хранятся как «текущий реестр», crates.io приводится к каноническому адресу, зависимость
    /// на любой другой реестр отвергается.
    pub fn index_metadata(&self, own_index: &str) -> Result<FormatMetadata, MetadataError> {
        let stored = self.json.clone().into_stored(own_index)?;
        let stored = serde_json::to_string(&stored).map_err(MetadataError::Serialize)?;
        Ok(FormatMetadata::new(stored))
    }
}

impl fmt::Debug for CargoPublish {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Тело крейта велико: в журнал идёт только длина.
        f.debug_struct("CargoPublish")
            .field("name", &self.name)
            .field("version", &self.version)
            .field("crate_file_len", &self.crate_file.len())
            .finish_non_exhaustive()
    }
}

/// Ошибка разбора публикации.
#[derive(Debug, thiserror::Error)]
pub enum PublishError {
    #[error("тело короче заявленных длин")]
    Truncated,
    #[error("лишние байты после крейта")]
    TrailingBytes,
    #[error("некорректный JSON публикации")]
    Json(#[source] serde_json::Error),
    #[error("некорректное имя пакета")]
    Name(#[source] NameError),
    #[error("некорректная версия")]
    Version(#[source] VersionError),
}

/// Стабильный код отказа публикации из-за зависимости на сторонний реестр.
pub const FOREIGN_REGISTRY_DEPENDENCY: &str = "foreign-registry-dependency";

/// Зависимость ссылается на реестр, из которого CRANE публиковать не даёт.
///
/// Значения взяты из недоверенного тела и обрезаны до 128 символов; в `Display` попадает
/// только имя зависимости.
#[derive(Debug, thiserror::Error)]
#[error(
    "зависимость {dependency:?} указывает на сторонний реестр: зависимости из сторонних \
     реестров запрещены, используйте зависимость с crates.io или из этого реестра"
)]
pub struct ForeignRegistry {
    pub dependency: String,
    pub registry: String,
}

/// Ошибка построения метаданных индекса.
#[derive(Debug, thiserror::Error)]
pub enum MetadataError {
    #[error(transparent)]
    Foreign(#[from] ForeignRegistry),
    // По построению недостижима (строковые ключи), но `unwrap` запрещён.
    #[error("метаданные не сериализуются")]
    Serialize(#[source] serde_json::Error),
}

/// Отрезает от начала кадр `u32 LE длина` + данные без копирования.
fn take_frame(buf: &mut Bytes) -> Result<Bytes, PublishError> {
    let (header, tail) = buf
        .split_first_chunk::<4>()
        .ok_or(PublishError::Truncated)?;
    let len = usize::try_from(u32::from_le_bytes(*header)).map_err(|_| PublishError::Truncated)?;
    if tail.len() < len {
        return Err(PublishError::Truncated);
    }
    drop(buf.split_to(4));
    Ok(buf.split_to(len))
}

/// Кадр cargo: `u32 LE` длина JSON, JSON, `u32 LE` длина крейта, крейт.
pub fn parse_publish(body: Bytes) -> Result<CargoPublish, PublishError> {
    let mut rest = body;
    let json = take_frame(&mut rest)?;
    let crate_file = take_frame(&mut rest)?;
    if !rest.is_empty() {
        return Err(PublishError::TrailingBytes);
    }
    let parsed: PublishJson = serde_json::from_slice(&json).map_err(PublishError::Json)?;
    let name = PackageName::parse(Format::Cargo, &parsed.name).map_err(PublishError::Name)?;
    let version = Version::parse(Format::Cargo, &parsed.vers).map_err(PublishError::Version)?;
    Ok(CargoPublish {
        name,
        version,
        crate_file,
        json: parsed,
    })
}
