//! Driving-адаптер HTTP: routers, middleware, problem+json.
//!
//! Форма ошибки определяется типом ошибки, а не точкой монтирования: cargo-клиенты
//! ждут `{"errors": [...]}`, остальные маршруты отвечают `application/problem+json`.

mod admin;
mod cargo;
mod context;
mod error;

pub use admin::admin_router;
pub use cargo::{CargoSettings, StaticToken, api_router};
pub use context::{ProxyParseError, TrustedProxies};
