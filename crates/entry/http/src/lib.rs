//! Driving-адаптер HTTP: routers, middleware, problem+json.
//!
//! Поведение формата (маршруты, форма тела ошибки, challenge, способ предъявить токен) задаёт таблица
//! `formats`: cargo-клиенты ждут `{"errors": [...]}`, npm — `{"error": …}`, остальные маршруты
//! отвечают `application/problem+json`.

mod admin;
mod api;
mod auth;
mod context;
mod error;
mod formats;
mod serve;

pub use admin::{admin_router, wizard_admin_router};
pub use api::{api_router, wizard_router};
pub use auth::StaticToken;
pub use context::{ACCESS_TARGET, ProxyParseError, TrustedProxies};
pub use formats::{FormatSettings, ProxySource, presence_origin, proxy_router, upstream_hosts};
pub use serve::EngineUpstreamNames;
