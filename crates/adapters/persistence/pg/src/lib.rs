//! Адаптер PostgreSQL: реализация портов `Catalog`, `SettingsStore`, `OnboardingStore` и миграции схемы.
//!
//! Запросы пишутся вручную (без макросов `sqlx`), строки отображаются в доменные типы в
//! `rows`. Схема развивается только вперёд по правилам из `migrations/README.md`.

mod catalog;
mod schema;
mod settings;

pub use catalog::PgCatalog;
pub use schema::{SchemaError, migrate};
pub use settings::{CryptoError, KeyError, PgOnboarding, PgSettings, SettingsKey};
