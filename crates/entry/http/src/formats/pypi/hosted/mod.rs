//! Hosted-репозитории: Simple API, файлы, загрузка twine, poetry и uv, временный маршрут yank.

mod read;
mod upload;
mod yank;

use std::sync::Arc;

use axum::Router;
use axum::routing::{get, post, put};
use crane_app::registry::{Coordinate, Registry};
use crane_domain::registry::{Ecosystem, Format};
use crane_format_pypi::PYPI;
use crane_ports::registry::{BlobStore, Catalog};

use crate::error::{ClientError, chain};
use crate::formats::FormatSettings;

/// Общее состояние обработчиков.
struct Shared<C, B> {
    registry: Arc<Registry<C, B>>,
    settings: FormatSettings,
}

/// Маршруты hosted-репозитория без слоя аутентификации: его накладывает `api_router`.
///
/// Корня `simple/` в v0.3.0 нет: клиентам он для установки не нужен, а список проектов
/// репозитория требует нового метода каталога.
pub(crate) fn hosted_router<C: Catalog + 'static, B: BlobStore + 'static>(
    registry: Arc<Registry<C, B>>,
    settings: &FormatSettings,
) -> Router {
    let shared = Arc::new(Shared {
        registry,
        settings: settings.clone(),
    });
    // twine и poetry падают на любой 3xx, а адрес без завершающего `/` — самая частая ошибка
    // настройки: оба написания обслуживаются одним обработчиком без редиректа.
    Router::new()
        .route("/simple/{name}/", get(read::project))
        .route("/simple/{name}", get(read::project_without_slash))
        .route("/files/{name}/{version}/{filename}", get(read::file))
        .route("/legacy", post(upload::upload))
        .route("/legacy/", post(upload::upload))
        // FIXME(v0.4.0): временный маршрут формата; операция переезжает в admin API yankVersion (docs/api/admin.openapi.yaml)
        .route(
            "/packages/{name}/versions/{version}/yank",
            put(yank::yank).delete(yank::unyank),
        )
        .fallback(pypi_not_found)
        .with_state(shared)
}

async fn pypi_not_found() -> ClientError {
    ClientError::not_found()
}

/// Координата версии из сегментов пути. Версия — любое написание PEP 440: каталог ищет по
/// канонической форме.
fn coordinate(repo: String, name: &str, version: &str) -> Result<Coordinate, ClientError> {
    let package = PYPI
        .parse_name(name)
        .map_err(|e| ClientError::bad_request(chain(&e)))?;
    let version = PYPI
        .parse_version(version)
        .map_err(|e| ClientError::bad_request(chain(&e)))?;
    Ok(Coordinate {
        format: Format::PyPi,
        repository: repo,
        package,
        version,
    })
}
