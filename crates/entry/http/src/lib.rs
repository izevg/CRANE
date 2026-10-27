//! Driving-адаптер HTTP: routers, middleware, problem+json.
//!
//! Форма тела ошибки — свойство точки монтирования (`Dialect`): cargo-клиенты ждут `{"errors": [...]}`,
//! npm — `{"error": …}`, остальные маршруты отвечают `application/problem+json`.

mod admin;
mod auth;
mod cargo;
mod context;
mod error;
mod serve;

pub use admin::admin_router;
pub use auth::StaticToken;
pub use cargo::{CargoProxyOrigins, CargoSettings, api_router, cargo_proxy_router};
pub use context::{ACCESS_TARGET, ProxyParseError, TrustedProxies};
