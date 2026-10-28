//! Driving-адаптер HTTP: routers, middleware, problem+json.
//!
//! Форма тела ошибки — свойство точки монтирования (`Dialect`): cargo-клиенты ждут `{"errors": [...]}`,
//! npm — `{"error": …}`, остальные маршруты отвечают `application/problem+json`.

mod admin;
mod api;
mod auth;
mod cargo;
mod context;
mod error;
mod npm;
mod serve;

pub use admin::admin_router;
pub use api::{FormatSettings, ProxyRouters, api_router};
pub use auth::StaticToken;
pub use cargo::{CargoProxyOrigins, CargoSettings, cargo_proxy_router};
pub use context::{ACCESS_TARGET, ProxyParseError, TrustedProxies};
pub use npm::{NpmSettings, npm_proxy_router};
pub use serve::EngineUpstreamNames;
