//! Исполняемая спецификация портов: один набор сценариев для всех адаптеров.
//!
//! Каждая функция гоняет сценарии, каждый на свежем экземпляре адаптера из `new`.
//! Исполнитель (runtime) выбирает вызывающий тест.

mod blob;
mod catalog;
mod namespace;
mod onboarding;
mod proxy;
mod settings;

pub use blob::blob_store_contract;
pub use catalog::catalog_contract;
pub use namespace::namespace_contract;
pub use onboarding::onboarding_store_contract;
pub use proxy::{fetch_leases_contract, proxy_cache_contract};
pub use settings::settings_store_contract;
