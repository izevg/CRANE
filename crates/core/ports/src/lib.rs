//! Порты ядра: трейты, через которые use cases обращаются к внешнему миру.
//!
//! Асинхронные порты (`BlobStore`, `Catalog`) не dyn-совместимы: `app` обобщён по ним,
//! а зависимости собирает `entry`. `Clock` и `IdGen` синхронны и dyn-совместимы.

pub mod registry;
pub mod system;

#[cfg(feature = "contract")]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "исполняемая спецификация порта: падение = нарушение контракта"
)]
pub mod contract;
