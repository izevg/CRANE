//! Предметная область CRANE: сущности, state machines, ошибки.
//!
//! Слой не зависит ни от одного другого крейта проекта и не выполняет IO.

pub mod access;
pub mod audit;
pub mod onboarding;
pub mod proxy;
pub mod registry;
pub mod settings;
