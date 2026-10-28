//! Метаданные, сохраняемые при публикации, и сборка NDJSON-индекса из них.

use std::collections::BTreeMap;

use crane_domain::registry::PackageVersion;
use serde::{Deserialize, Serialize};

use crate::api::asset_path;

/// Что лежит в `FormatMetadata` версии: часть для индекса и описательная часть.
/// Только запись: путь индекса читает `IndexView`, описательная часть ему не нужна.
#[derive(Debug, Serialize)]
pub(crate) struct StoredMeta {
    pub(crate) index: IndexMeta,
    pub(crate) about: About,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct IndexMeta {
    pub(crate) deps: Vec<IndexDep>,
    pub(crate) features: BTreeMap<String, Vec<String>>,
    pub(crate) links: Option<String>,
    pub(crate) rust_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct IndexDep {
    pub(crate) name: String,
    pub(crate) req: String,
    pub(crate) features: Vec<String>,
    pub(crate) optional: bool,
    pub(crate) default_features: bool,
    pub(crate) target: Option<String>,
    pub(crate) kind: String,
    pub(crate) registry: Option<String>,
    pub(crate) package: Option<String>,
}

/// Описательные поля для веб-интерфейса; `readme` не храним из-за размера.
#[derive(Debug, Serialize)]
pub(crate) struct About {
    pub(crate) description: Option<String>,
    pub(crate) license: Option<String>,
    pub(crate) license_file: Option<String>,
    pub(crate) homepage: Option<String>,
    pub(crate) repository: Option<String>,
    pub(crate) documentation: Option<String>,
    pub(crate) keywords: Vec<String>,
    pub(crate) categories: Vec<String>,
}

/// Читаемая часть сохранённых метаданных: serde пропускает `about`, и он не
/// разбирается на каждый запрос индекса.
#[derive(Deserialize)]
struct IndexView {
    index: IndexMeta,
}

/// Ошибка сборки индекса.
#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// Сохранённые метаданные версии не разбираются.
    #[error("повреждённые метаданные версии")]
    Metadata(#[source] serde_json::Error),
    /// У версии нет файла `.crate`, из которого берётся контрольная сумма.
    #[error("у версии {package} {version} нет файла .crate")]
    MissingCrateFile { package: String, version: String },
}

/// Строка индекса. Расширенный синтаксис features допустим в `features` по документации
/// cargo; `v`, `features2` и `pubtime` не нужны, sparse требует cargo >= 1.68.
#[derive(Serialize)]
struct IndexLine<'a> {
    name: &'a str,
    vers: &'a str,
    deps: &'a [IndexDep],
    cksum: String,
    features: &'a BTreeMap<String, Vec<String>>,
    yanked: bool,
    links: Option<&'a str>,
    rust_version: Option<&'a str>,
}

/// NDJSON: по строке на версию, каждая оканчивается '\n'. Порядок как во входе.
pub fn render_index(versions: &[PackageVersion]) -> Result<String, IndexError> {
    let mut out = String::new();
    for v in versions {
        let meta: IndexView =
            serde_json::from_str(v.metadata.as_str()).map_err(IndexError::Metadata)?;
        let path = asset_path(&v.package, &v.version);
        let asset = v.assets.iter().find(|a| a.path == path).ok_or_else(|| {
            IndexError::MissingCrateFile {
                package: v.package.as_written().to_owned(),
                version: v.version.as_written().to_owned(),
            }
        })?;
        let line = IndexLine {
            name: v.package.as_written(),
            vers: v.version.as_written(),
            deps: &meta.index.deps,
            cksum: asset.content.sha256.to_string(),
            features: &meta.index.features,
            yanked: v.yanked,
            links: meta.index.links.as_deref(),
            rust_version: meta.index.rust_version.as_deref(),
        };
        let json = serde_json::to_string(&line).map_err(IndexError::Metadata)?;
        out.push_str(&json);
        out.push('\n');
    }
    Ok(out)
}
