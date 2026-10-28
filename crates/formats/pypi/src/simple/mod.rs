//! Страница проекта Simple API: одна модель и два представления, HTML (PEP 503) и JSON (PEP 691).

mod html;
mod json;

use std::time::SystemTime;

use crane_domain::registry::{PackageName, PackageVersion, Sha256};

use crate::upload::VersionMetadata;

const JSON_TYPE: &str = "application/vnd.pypi.simple.v1+json";
const LATEST_JSON_TYPE: &str = "application/vnd.pypi.simple.latest+json";
/// Параметр не добавляем: poetry узнаёт JSON только по точному значению заголовка.
const HTML_TYPE: &str = "text/html; charset=utf-8";

/// Страница проекта: имя, версии и файлы. Ссылки на файлы уже наши и абсолютные: рендер не знает,
/// откуда файл.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: PackageName,
    pub versions: Vec<String>,
    pub files: Vec<DistFile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistFile {
    pub filename: String,
    pub url: String,
    pub sha256: Sha256,
    pub requires_python: Option<String>,
    pub yanked: Yanked,
    pub size: u64,
    pub upload_time: Option<SystemTime>,
}

/// PEP 592: файл отозван, причина необязательна.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Yanked {
    No,
    Yes(Option<String>),
}

/// Представление страницы.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Html,
    Json,
}

impl View {
    /// JSON — только если клиент назвал его явно (pip, uv и poetry ставят его первым), иначе HTML,
    /// который годится любому. Весов `q` и ответа 406 нет: сервер вправе выбрать любой из
    /// запрошенных типов, а HTML подходит всем, кто не просит JSON.
    #[must_use]
    pub fn from_accept(header: Option<&str>) -> Self {
        let asks_json = header.is_some_and(|accept| {
            let accept = accept.to_ascii_lowercase();
            accept.contains(JSON_TYPE) || accept.contains(LATEST_JSON_TYPE)
        });
        if asks_json { Self::Json } else { Self::Html }
    }
}

#[must_use]
pub const fn content_type(view: View) -> &'static str {
    match view {
        View::Html => HTML_TYPE,
        View::Json => JSON_TYPE,
    }
}

#[must_use]
pub fn render(project: &Project, view: View) -> String {
    match view {
        View::Html => html::render(project),
        View::Json => json::render(project),
    }
}

/// Страница из каталога: каждый файл версии — файл страницы. `base` — адрес точки монтирования
/// репозитория без завершающего `/`.
///
/// `requires-python` и время загрузки берутся у версии: метаданные версии пишет первый файл, а
/// `published_at` — момент её создания. Для файлов, прикреплённых позже, время будет раньше
/// настоящего; это честное упрощение при одном наборе метаданных на версию.
#[must_use]
pub fn hosted(name: &PackageName, versions: &[PackageVersion], base: &str) -> Project {
    let files = versions
        .iter()
        .flat_map(|version| {
            let requires_python = VersionMetadata::read(&version.metadata).requires_python;
            version.assets.iter().map(move |asset| DistFile {
                filename: asset.path.clone(),
                url: format!(
                    "{base}/files/{}/{}/{}",
                    name.normalized(),
                    version.version.canonical(),
                    asset.path
                ),
                sha256: asset.sha256,
                requires_python: requires_python.clone(),
                yanked: if version.yanked {
                    Yanked::Yes(None)
                } else {
                    Yanked::No
                },
                size: asset.size,
                upload_time: Some(version.published_at),
            })
        })
        .collect();
    Project {
        name: name.clone(),
        versions: versions
            .iter()
            .map(|version| version.version.canonical().to_owned())
            .collect(),
        files,
    }
}
