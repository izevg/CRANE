//! Исполняемая спецификация портов: один набор сценариев для всех адаптеров.
//!
//! Каждая функция гоняет сценарии, каждый на свежем экземпляре адаптера из `new`.
//! Исполнитель (runtime) выбирает вызывающий тест.

mod blob;
mod catalog;

pub use blob::blob_store_contract;
pub use catalog::catalog_contract;
