//! Адаптер SIEM: события аудита в именах полей OCSF 1.3.
//!
//! В v0.4.0 журнал PostgreSQL заменит этот приёмник для действий записи (та же транзакция,
//! что и само действие); приёмник останется для событий без транзакции. Сопоставление
//! события с OCSF (`ocsf`) переиспользует экспорт в SIEM v1.1.

mod ocsf;
mod sink;

pub use ocsf::{OcsfRecord, OcsfUser, ocsf};
pub use sink::{AUDIT_TARGET, OcsfLogSink};
