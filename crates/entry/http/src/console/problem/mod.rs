//! Отказы use cases настроек в `problem+json`. Тип проблемы называет причину, статус — класс отказа.

use std::error::Error;

use axum::http::StatusCode;
use crane_app::settings::SettingError;
use serde_json::json;

use crate::error::{Problem, ProblemKind, chain};

const UNREACHABLE_DETAIL: &str = "хранилище настроек временно недоступно";

/// 503 для обоих портов (настройки и первый запуск): повтор позже имеет смысл. `Retry-After: 10`
/// ставит единый рендер всех 503. Цепочка причин (хост, порт БД) остаётся в журнале.
pub(crate) fn unreachable(error: &(dyn Error + 'static)) -> Problem {
    tracing::warn!(error = %chain(error), "хранилище настроек недоступно");
    Problem::new(
        StatusCode::SERVICE_UNAVAILABLE,
        UNREACHABLE_DETAIL,
        ProblemKind::SettingsUnreachable,
    )
}

impl From<SettingError> for Problem {
    fn from(error: SettingError) -> Self {
        match &error {
            SettingError::Unknown => Self::new(
                StatusCode::NOT_FOUND,
                "настройка не найдена",
                ProblemKind::Blank,
            ),
            SettingError::Locked { var } => Self::new(
                StatusCode::CONFLICT,
                format!("настройка задана переменной {var}: уберите её из окружения"),
                ProblemKind::SettingLocked,
            )
            .with("var", json!(var)),
            SettingError::NotPersistent => Self::new(
                StatusCode::CONFLICT,
                "настройки не сохраняются: хранилище работает в памяти",
                ProblemKind::SettingsNotPersistent,
            ),
            SettingError::Invalid(invalid) => {
                let errors: Vec<_> = invalid
                    .errors
                    .iter()
                    .map(|item| json!({"key": item.key, "detail": item.detail}))
                    .collect();
                Self::new(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    error.to_string(),
                    ProblemKind::SettingInvalid,
                )
                .with("errors", json!(errors))
            }
            SettingError::Conflict => Self::new(
                StatusCode::PRECONDITION_FAILED,
                "версия настройки устарела: перечитайте её",
                ProblemKind::Blank,
            ),
            SettingError::SecretsUnavailable => Self::new(
                StatusCode::UNPROCESSABLE_ENTITY,
                "хранение секретов недоступно: задайте ключ шифрования",
                ProblemKind::SecretsUnavailable,
            ),
            SettingError::Unreachable(_) => unreachable(&error),
            SettingError::Backend(_) => Self::internal(&error),
        }
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
