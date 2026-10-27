//! Заимствованная модель packument: поля остаются сырыми фрагментами входа, `Value` не материализуется.

use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::value::{RawValue, to_raw_value};

use super::DocError;

/// Объект JSON: ключи `Cow` (экранированный ключ нельзя занять из входа без копии).
pub(crate) type Fields<'a> = BTreeMap<Cow<'a, str>, Raw<'a>>;

const VERSIONS: &str = "versions";

/// Сырое значение JSON: кусок входа либо собранное заново (подменённые поля).
#[derive(Debug, Clone)]
pub(crate) enum Raw<'a> {
    Borrowed(&'a RawValue),
    Owned(Box<RawValue>),
}

impl<'de: 'a, 'a> Deserialize<'de> for Raw<'a> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        <&'de RawValue>::deserialize(deserializer).map(Raw::Borrowed)
    }
}

impl Serialize for Raw<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Borrowed(raw) => raw.serialize(serializer),
            Self::Owned(raw) => raw.serialize(serializer),
        }
    }
}

impl Raw<'_> {
    pub(crate) fn owned(value: &impl Serialize) -> Result<Self, serde_json::Error> {
        to_raw_value(value).map(Raw::Owned)
    }

    pub(crate) fn get(&self) -> &str {
        match self {
            Self::Borrowed(raw) => raw.get(),
            Self::Owned(raw) => raw.get(),
        }
    }

    /// Значение как строка JSON; любой другой тип — `None`.
    pub(crate) fn as_string(&self) -> Option<String> {
        serde_json::from_str(self.get()).ok()
    }

    /// Значение как объект; любой другой тип — `None`.
    pub(crate) fn as_object(&self) -> Option<Fields<'_>> {
        serde_json::from_str(self.get()).ok()
    }
}

/// Packument: верхний уровень и версии как карты сырых полей.
#[derive(Debug, Default)]
pub(crate) struct Packument<'a> {
    pub(crate) fields: Fields<'a>,
    pub(crate) versions: BTreeMap<Cow<'a, str>, Fields<'a>>,
}

impl<'a> Packument<'a> {
    pub(crate) fn parse(raw: &'a [u8]) -> Result<Self, DocError> {
        let mut fields: Fields<'a> = serde_json::from_slice(raw)?;
        let versions = match fields.remove(VERSIONS) {
            Some(Raw::Borrowed(raw)) => serde_json::from_str(raw.get())?,
            Some(Raw::Owned(_)) | None => BTreeMap::new(),
        };
        Ok(Self { fields, versions })
    }

    pub(crate) fn to_vec(&self) -> Result<Vec<u8>, DocError> {
        Ok(serde_json::to_vec(self)?)
    }
}

impl Serialize for Packument<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.fields.len() + 1))?;
        for (key, value) in &self.fields {
            map.serialize_entry(key, value)?;
        }
        map.serialize_entry(VERSIONS, &self.versions)?;
        map.end()
    }
}
