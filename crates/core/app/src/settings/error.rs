use crane_ports::settings::{Invalid, SettingsError};

/// Отказ use case настроек. Коды стабильны: по ним консоль и журнал аудита различают причины.
#[derive(Debug, thiserror::Error)]
pub enum SettingError {
    /// Нет такого ключа или он не для консоли.
    #[error("настройка не найдена")]
    Unknown,
    /// Ключ задан переменной окружения или файлом: консоль его не меняет.
    #[error("настройка задана переменной {var}")]
    Locked { var: String },
    /// Режим memory: записи некуда сохраняться.
    #[error("настройки не сохраняются: хранилище работает в памяти")]
    NotPersistent,
    /// Значения подробностей нет намеренно: в журнал попадает только Display.
    #[error("значение не прошло проверку секции")]
    Invalid(Invalid),
    #[error("конфликт версии настройки")]
    Conflict,
    #[error("хранение секретов недоступно: нет ключа шифрования")]
    SecretsUnavailable,
    #[error("хранилище настроек недоступно")]
    Unreachable(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("ошибка хранилища настроек")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl SettingError {
    /// Стабильный код отказа для журнала и ответа API.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Unknown => "setting-unknown",
            Self::Locked { .. } => "setting-locked",
            Self::NotPersistent => "settings-not-persistent",
            Self::Invalid(_) => "setting-invalid",
            Self::Conflict => "setting-conflict",
            Self::SecretsUnavailable => "secrets-unavailable",
            Self::Unreachable(_) => "settings-unreachable",
            Self::Backend(_) => "settings-backend",
        }
    }

    /// Переменная, из-за которой запись запрещена; только у `Locked`.
    pub(super) fn env_var(&self) -> Option<String> {
        match self {
            Self::Locked { var } => Some(var.clone()),
            _ => None,
        }
    }
}

// Единственное место, где ошибки порта превращаются в ошибки use case.
impl From<SettingsError> for SettingError {
    fn from(error: SettingsError) -> Self {
        match error {
            SettingsError::Conflict => Self::Conflict,
            SettingsError::SecretsUnavailable => Self::SecretsUnavailable,
            SettingsError::Unreachable(source) => Self::Unreachable(source),
            SettingsError::Backend(source) => Self::Backend(source),
        }
    }
}
