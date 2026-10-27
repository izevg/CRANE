//! JSON публикации от cargo и его преобразование в сохраняемые метаданные.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::index::{About, IndexDep, IndexMeta, StoredMeta};

const fn default_true() -> bool {
    true
}

// Неизвестные поля не запрещены: cargo добавляет новые (readme, badges, authors и др.).
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
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

impl PublishDep {
    fn into_index(self) -> IndexDep {
        // В индексе `name` — имя в Cargo.toml зависимого, `package` — настоящее имя крейта.
        let (name, package) = match self.explicit_name_in_toml {
            Some(alias) => (alias, Some(self.name)),
            None => (self.name, None),
        };
        IndexDep {
            name,
            req: self.version_req,
            features: self.features,
            optional: self.optional,
            default_features: self.default_features,
            target: self.target,
            kind: self.kind.unwrap_or_else(|| "normal".to_owned()),
            registry: self.registry,
            package,
        }
    }
}

impl PublishJson {
    pub(super) fn into_stored(self) -> StoredMeta {
        StoredMeta {
            index: IndexMeta {
                deps: self.deps.into_iter().map(PublishDep::into_index).collect(),
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
        }
    }
}
