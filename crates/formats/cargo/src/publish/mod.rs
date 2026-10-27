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
    pub metadata: FormatMetadata,
    pub crate_file: Bytes,
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
    let stored = serde_json::to_string(&parsed.into_stored()).map_err(PublishError::Json)?;
    Ok(CargoPublish {
        name,
        version,
        metadata: FormatMetadata::new(stored),
        crate_file,
    })
}
