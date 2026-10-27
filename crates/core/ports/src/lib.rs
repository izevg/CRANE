//! Порты ядра: трейты, через которые use cases обращаются к внешнему миру.
//!
//! Асинхронные порты (`BlobStore`, `Catalog`) не dyn-совместимы: `app` обобщён по ним,
//! а зависимости собирает `entry`. `Clock` и `IdGen` синхронны и dyn-совместимы.

mod registry;
mod system;

// Контракт не `#[cfg(test)]`, поэтому линты на паники приходится снимать здесь, в одном месте.
#[cfg(feature = "contract")]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "исполняемая спецификация порта: падение = нарушение контракта"
)]
pub mod contract;

pub use registry::{
    BlobError, BlobStore, ByteStream, Catalog, CatalogError, Digests, NewVersion, Staged,
    StagingKey,
};
pub use system::{Clock, IdGen};
