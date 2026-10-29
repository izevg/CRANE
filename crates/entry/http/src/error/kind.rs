//! Тип проблемы RFC 9457 для ошибок вне форматов.

/// Тип проблемы RFC 9457 (поле `type` вне форматов). Задаётся тем, кто создаёт ошибку,
/// а не выводится из статуса: один и тот же 503 бывает «хранилище не отвечает» и «не готов».
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProblemKind {
    Blank,
    StorageUnreachable,
    /// Первый запуск не завершён: протоколы пакетов недоступны, пока мастер не закончен (20.2).
    NotConfigured,
}

impl ProblemKind {
    pub(crate) const fn uri(self) -> &'static str {
        match self {
            Self::Blank => "about:blank",
            Self::StorageUnreachable => "urn:crane:problem:storage-unreachable",
            Self::NotConfigured => "urn:crane:problem:not-configured",
        }
    }
}
