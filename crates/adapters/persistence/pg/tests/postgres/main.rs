//! Поведение, специфичное для PostgreSQL: атомарность, гонки, точность времени, миграции.

#[allow(
    dead_code,
    reason = "помощники общие для нескольких тестовых бинарников"
)]
#[path = "../support/mod.rs"]
mod support;

mod assets;
mod migrate;
mod onboarding;
mod publish;
mod settings;
mod versions;

/// Число конкурентов в гонках: достаточно, чтобы окно между проверкой и записью пересеклось.
const CONTENDERS: u64 = 16;
