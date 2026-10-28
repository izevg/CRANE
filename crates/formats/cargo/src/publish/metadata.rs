//! JSON публикации от cargo и его преобразование в сохраняемые метаданные.

use std::collections::BTreeMap;

use serde::Deserialize;

use super::ForeignRegistry;
use crate::index::{About, IndexDep, IndexMeta, StoredMeta};

/// Предел длины недоверенных значений в ошибке: они уходят в ответ и журнал аудита.
const MAX_ECHO_CHARS: usize = 128;
/// Адрес crates.io в той форме, в какой cargo сам пишет его при publish.
const CRATES_IO: &str = "https://github.com/rust-lang/crates.io-index";
/// Тот же crates.io в sparse-форме: в индексе хранится только каноническая.
const CRATES_IO_SPARSE: &str = "sparse+https://index.crates.io/";

const fn default_true() -> bool {
    true
}

// Неизвестные поля не запрещены: cargo добавляет новые (readme, badges, authors и др.).
#[derive(Debug, Clone, Deserialize)]
pub(super) struct PublishJson {
    pub(super) name: String,
    pub(super) vers: String,
    #[serde(default)]
    deps: Vec<PublishDep>,
    #[serde(default)]
    features: BTreeMap<String, Vec<String>>,
    links: Option<String>,
    rust_version: Option<String>,
    description: Option<String>,
    documentation: Option<String>,
    homepage: Option<String>,
    repository: Option<String>,
    license: Option<String>,
    license_file: Option<String>,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    categories: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct PublishDep {
    name: String,
    version_req: String,
    #[serde(default)]
    features: Vec<String>,
    #[serde(default)]
    optional: bool,
    #[serde(default = "default_true")]
    default_features: bool,
    target: Option<String>,
    kind: Option<String>,
    registry: Option<String>,
    explicit_name_in_toml: Option<String>,
}

/// Приводит `registry` зависимости к виду, безопасному для индекса.
///
/// Поле попадает в индекс и задаёт, откуда потребитель возьмёт зависимость: сторонний адрес
/// увёл бы сборки мимо CRANE. Поэтому «свой» реестр сворачивается в `None` (в индексе это
/// «текущий реестр»), crates.io канонизируется, а всё остальное — отказ с исходным значением.
fn dep_registry(raw: Option<String>, own_index: &str) -> Result<Option<String>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let is_own = raw == own_index || Some(raw.as_str()) == own_index.strip_suffix('/');
    if is_own {
        Ok(None)
    } else if raw == CRATES_IO || raw == CRATES_IO_SPARSE {
        Ok(Some(CRATES_IO.to_owned()))
    } else {
        Err(raw)
    }
}

fn clip(value: &str) -> String {
    value.chars().take(MAX_ECHO_CHARS).collect()
}

impl PublishDep {
    fn into_index(self, own_index: &str) -> Result<IndexDep, ForeignRegistry> {
        let registry =
            dep_registry(self.registry, own_index).map_err(|registry| ForeignRegistry {
                dependency: clip(&self.name),
                registry: clip(&registry),
            })?;
        // В индексе `name` — имя в Cargo.toml зависимого, `package` — настоящее имя крейта.
        let (name, package) = match self.explicit_name_in_toml {
            Some(alias) => (alias, Some(self.name)),
            None => (self.name, None),
        };
        Ok(IndexDep {
            name,
            req: self.version_req,
            features: self.features,
            optional: self.optional,
            default_features: self.default_features,
            target: self.target,
            kind: self.kind.unwrap_or_else(|| "normal".to_owned()),
            registry,
            package,
        })
    }
}

impl PublishJson {
    pub(super) fn into_stored(self, own_index: &str) -> Result<StoredMeta, ForeignRegistry> {
        let deps = self
            .deps
            .into_iter()
            .map(|dep| dep.into_index(own_index))
            .collect::<Result<_, _>>()?;
        Ok(StoredMeta {
            index: IndexMeta {
                deps,
                features: self.features,
                links: self.links,
                rust_version: self.rust_version,
            },
            about: About {
                description: self.description,
                license: self.license,
                license_file: self.license_file,
                homepage: self.homepage,
                repository: self.repository,
                documentation: self.documentation,
                keywords: self.keywords,
                categories: self.categories,
            },
        })
    }
}
