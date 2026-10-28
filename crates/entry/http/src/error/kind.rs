//! Тип проблемы RFC 9457 для ошибок вне форматов.

/// Тип проблемы RFC 9457 (поле `type` вне форматов). Задаётся тем, кто создаёт ошибку,
/// а не выводится из статуса: один и тот же 503 бывает «хранилище не отвечает» и «не готов».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProblemKind {
    Blank,
    StorageUnreachable,
    // Консоль (Admin API): `type` называет причину, по которой UI выбирает текст и действие.
    SettingLocked,
    SettingsNotPersistent,
    SettingInvalid,
    SecretsUnavailable,
    SettingsUnreachable,
    PreconditionRequired,
}

impl ProblemKind {
    pub(crate) const fn uri(self) -> &'static str {
        match self {
            Self::Blank => "about:blank",
            Self::StorageUnreachable => "urn:crane:problem:storage-unreachable",
            Self::SettingLocked => "urn:crane:problem:setting-locked",
            Self::SettingsNotPersistent => "urn:crane:problem:settings-not-persistent",
            Self::SettingInvalid => "urn:crane:problem:setting-invalid",
            Self::SecretsUnavailable => "urn:crane:problem:secrets-unavailable",
            Self::SettingsUnreachable => "urn:crane:problem:settings-unreachable",
            Self::PreconditionRequired => "urn:crane:problem:precondition-required",
        }
    }
}
