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
    /// Запись или удаление отклонены. Причина и код лежат в `Outcome::Failure`; сырого значения
    /// в событии нет: опечатка в имени ключа не должна унести токен в журнал.
    SettingRejected {
        key: String,
        /// Переменная, задавшая ключ, если отказ из-за блокировки.
        env_var: Option<String>,
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

/// Что изменилось в настройке. Для секрета значений нет вовсе: ни старого, ни нового, только адрес.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingChange {
    /// `old: None` — настройка создана.
    Plain { old: Option<String>, new: String },
    Secret {
        /// Перезаписан секрет, который не открывался (потерян или сменён ключ, повреждена строка).
        replaced_unreadable: bool,
        /// Адрес прежнего и нового значения без учётных данных (`scheme://host[:port]`), если секрет
        /// похож на URL. Смена адреса кэша или хранилища — способ подмены данных, её должно быть видно.
        old_locator: Option<String>,
        new_locator: Option<String>,
    },
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
