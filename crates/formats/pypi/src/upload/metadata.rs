use crane_domain::registry::FormatMetadata;
use serde::Deserialize;
use serde_json::{Map, Value};

/// Метаданные версии: только то, что читает рендер страницы, плюс лицензия (её заберёт слой
/// безопасности). Остальные поля формы загрузки не хранятся: источник истины — файл, а не форма.
#[derive(Debug, Default, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct VersionMetadata {
    #[serde(default)]
    pub(crate) requires_python: Option<String>,
    #[serde(default)]
    pub(crate) license_expression: Option<String>,
}

impl VersionMetadata {
    pub(crate) fn to_format(&self) -> FormatMetadata {
        let mut stored = Map::new();
        if let Some(requires_python) = &self.requires_python {
            stored.insert(
                "requires_python".into(),
                Value::from(requires_python.as_str()),
            );
        }
        if let Some(license) = &self.license_expression {
            stored.insert("license_expression".into(), Value::from(license.as_str()));
        }
        FormatMetadata::new(Value::Object(stored).to_string())
    }

    /// Нечитаемые метаданные не прячут версию от клиентов: страница строится без `requires-python`.
    pub(crate) fn read(metadata: &FormatMetadata) -> Self {
        serde_json::from_str(metadata.as_str()).unwrap_or_default()
    }
}
