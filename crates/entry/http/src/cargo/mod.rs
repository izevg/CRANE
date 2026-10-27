//! Маршруты cargo sparse-реестра под `/cargo/{repo}`: hosted и proxy-репозитории.

mod hosted;
mod proxy;

pub(crate) use hosted::hosted_router;
pub use proxy::{CargoProxyOrigins, cargo_proxy_router};

use crate::error::ClientError;

/// Точка монтирования cargo-маршрутов; от неё же строится база в `config.json`.
pub(crate) const CARGO_PREFIX: &str = "/cargo";
/// Тип тела sparse-индекса.
const INDEX_TYPE: &str = "text/plain; charset=utf-8";

/// Настройки cargo-маршрутов.
#[derive(Debug, Clone)]
pub struct CargoSettings {
    /// Внешний адрес без завершающего '/': клиент получает его в `config.json`.
    pub public_url: String,
    /// Предел тела публикации: оно буферизуется в памяти целиком.
    pub max_publish_bytes: usize,
}

async fn cargo_not_found() -> ClientError {
    ClientError::not_found()
}
