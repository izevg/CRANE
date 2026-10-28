//! Маршруты cargo sparse-реестра под `/cargo/{repo}`: hosted и proxy-репозитории.

mod hosted;
mod proxy;

pub(super) use hosted::hosted_router;
pub(super) use proxy::{presence_origin, proxy_router};

use crate::error::ClientError;

/// Тип тела sparse-индекса.
const INDEX_TYPE: &str = "text/plain; charset=utf-8";

async fn cargo_not_found() -> ClientError {
    ClientError::not_found()
}
