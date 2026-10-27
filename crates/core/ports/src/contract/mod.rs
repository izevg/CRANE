//! Исполняемая спецификация портов: один набор сценариев для всех адаптеров.
//!
//! Каждая функция гоняет сценарии, каждый на свежем экземпляре адаптера из `new`.
//! Исполнитель (runtime) выбирает вызывающий тест.

mod blob;
mod catalog;
mod proxy;

pub use blob::blob_store_contract;
pub use catalog::catalog_contract;
pub use proxy::{fetch_leases_contract, proxy_cache_contract};
