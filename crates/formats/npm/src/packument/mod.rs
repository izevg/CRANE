//! Packument — документ пакета: чтение upstream, сборка для hosted, полный и сокращённый виды.

mod model;
mod project;
mod render;

use crane_domain::registry::NameError;

pub(crate) use model::{Fields, Raw};
pub use render::{effective_tags, render_hosted, render_upstream};

const FULL_TYPE: &str = "application/json";
const ABBREVIATED_TYPE: &str = "application/vnd.npm.install-v1+json";

/// Вид документа: полный или сокращённый (corgi), которым клиенты ставят пакеты.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Full,
    Abbreviated,
}

impl View {
    /// Сокращённый вид — только если клиент его явно просит.
    #[must_use]
    pub fn from_accept(header: Option<&str>) -> Self {
        match header {
            Some(accept) if accept.contains(ABBREVIATED_TYPE) => Self::Abbreviated,
            _ => Self::Full,
        }
    }
}

#[must_use]
pub const fn content_type(view: View) -> &'static str {
    match view {
        View::Full => FULL_TYPE,
        View::Abbreviated => ABBREVIATED_TYPE,
    }
}

/// Внешний URL точки монтирования без хвостового `/`, например `https://h/npm/repo`.
#[derive(Debug, Clone, Copy)]
pub struct TarballBase<'a>(pub &'a str);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DocError {
    #[error("некорректный JSON документа: {0}")]
    Json(String),
    #[error("в документе нет имени пакета")]
    MissingName,
    #[error("некорректное имя пакета в документе")]
    Name(#[source] NameError),
    #[error("имя в документе не совпадает с запрошенным пакетом")]
    NameMismatch,
    #[error("в документе нет ни одной версии")]
    NoVersions,
}

impl From<serde_json::Error> for DocError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error.to_string())
    }
}
