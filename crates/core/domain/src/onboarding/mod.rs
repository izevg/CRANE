//! Первый запуск: состояние мастера, хэш bootstrap-токена и отчёты проб (только данные).

mod probe;
mod state;
mod token;

pub use probe::{ProbeCheck, ProbeReport, ProbeStatus};
pub use state::Onboarding;
pub use token::TokenHash;
