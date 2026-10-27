//! Порты ядра: трейты, через которые use cases обращаются к внешнему миру.
//!
//! Асинхронные порты (`BlobStore`, `Catalog`, `ProxyCache`, `FetchLeases`, `Upstream`)
//! не dyn-совместимы: `app` обобщён по ним, а зависимости собирает `entry`. `Clock`, `IdGen` и `AuditSink` синхронны и dyn-совместимы.

pub mod audit;
pub mod proxy;
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
