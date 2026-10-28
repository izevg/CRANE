//! Факты для журнала аудита.

use crate::access::{Principal, RequestContext};
use crate::registry::{Coordinate, Format, Namespace, PackageName, PackageRef};
use crate::settings::Version;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditAction {
    Authenticate,
    Publish(Coordinate),
    /// Файл, прикреплённый к версии: путь нужен, чтобы журнал ответил, кто подложил колесо.
    PublishFile {
        coordinate: Coordinate,
        path: String,
    },
    Yank(Coordinate),
    Unyank(Coordinate),
    Delete(Coordinate),
    Deprecate {
        coordinate: Coordinate,
        /// `None` снимает устаревание.
        message: Option<String>,
    },
    Tag {
        coordinate: Coordinate,
        tag: String,
    },
    Untag {
        package: PackageRef,
        tag: String,
    },
    /// Резервирование имён: конфигурационный namespace или первая публикация точного имени.
    Reserve(Namespace),
    /// Прокси отказался отдавать внутреннее имя из публичного upstream.
    ShadowUpstream {
        format: Format,
        package: PackageName,
    },
    SettingChanged {
        key: String,
        version: Version,
        change: SettingChange,
    },
    /// Запись отклонена: ключ задан переменной окружения или файлом.
    SettingRejectedLocked {
        key: String,
        env_var: String,
    },
    OnboardingStarted,
    OnboardingCompleted {
        via: OnboardingCredential,
    },
    OnboardingSkippedPreconfigured,
    OnboardingTokenRejected,
    OnboardingTokenRotated,
    /// Break-glass сброс мастера с хоста.
    OnboardingReset,
    /// Настройка удалена; значения удалённого в журнал не пишутся.
    SettingDeleted {
        key: String,
        secret: bool,
    },
}

/// Что изменилось в настройке. Для секрета значений нет вовсе: ни старого, ни нового.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingChange {
    /// `old: None` — настройка создана.
    Plain {
        old: Option<String>,
        new: String,
    },
    Secret,
}

/// Чем открыт вход в мастер.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingCredential {
    BootstrapToken,
    StaticToken,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Success,
    /// `code` — стабильный машиночитаемый код отказа по правилу; `None`, если правила нет.
    Failure {
        reason: String,
        code: Option<&'static str>,
    },
}

/// Факт для журнала. `event_id` и время ставит приёмник.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// `None` только у неуспешной `Authenticate`: субъект не установлен.
    pub actor: Option<Principal>,
    pub request: RequestContext,
    pub action: AuditAction,
    pub outcome: Outcome,
}
