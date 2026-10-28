use crane_domain::access::Caller;
use crane_domain::audit::{AuditAction, AuditEvent, Outcome, SettingChange};
use crane_domain::settings::{
    Kind, NewValue, Secret, SettingDef, Stored, StoredValue, Version, locator,
};
use crane_ports::settings::SettingsStore;

use super::{SettingError, Settings};

/// Успех записи: результат для вызывающего и событие, которое он породил.
struct Applied<T> {
    output: T,
    action: AuditAction,
}

impl<S: SettingsStore> Settings<S> {
    /// Записывает значение ключа. `expected = None` создаёт строку, `Some` обновляет по CAS.
    ///
    /// Каждый исход оставляет событие: успех — `SettingChanged`, отказ — `SettingRejected`.
    ///
    /// # Errors
    /// `Unknown`, `Locked`, `NotPersistent`, `Invalid`, а также ошибки хранилища.
    pub async fn put(
        &self,
        caller: &Caller,
        key: &str,
        raw: String,
        expected: Option<Version>,
    ) -> Result<Version, SettingError> {
        let result = self.write(key, raw, expected, caller).await;
        self.audited(caller, key, result)
    }

    /// Удаляет строку ключа по CAS: ключ возвращается к умолчанию после перезапуска.
    ///
    /// # Errors
    /// Те же, что у [`Self::put`]; удаление, ломающее секцию, отклоняется как `Invalid`.
    pub async fn delete(
        &self,
        caller: &Caller,
        key: &str,
        expected: Version,
    ) -> Result<(), SettingError> {
        let result = self.remove(key, expected).await;
        self.audited(caller, key, result)
    }

    async fn write(
        &self,
        key: &str,
        raw: String,
        expected: Option<Version>,
        caller: &Caller,
    ) -> Result<Applied<Version>, SettingError> {
        let def = self.writable(key)?;
        let rows = self.store.list().await?;
        let value = match def.kind {
            Kind::Secret => NewValue::Secret(Secret::new(raw)),
            _ => NewValue::Plain(raw),
        };
        self.schema
            .check(def.group, &rows, &[(key, Some(&value))])
            .map_err(SettingError::Invalid)?;
        let change = change_of(rows.get(key), &value);
        let version = self
            .store
            .put(key, value, expected, &caller.principal)
            .await?;
        Ok(Applied {
            output: version,
            action: AuditAction::SettingChanged {
                key: key.to_owned(),
                version,
                change,
            },
        })
    }

    async fn remove(&self, key: &str, expected: Version) -> Result<Applied<()>, SettingError> {
        let def = self.writable(key)?;
        let rows = self.store.list().await?;
        // Удаление — тоже запись: без проверки консоль ломала бы следующий старт.
        self.schema
            .check(def.group, &rows, &[(key, None)])
            .map_err(SettingError::Invalid)?;
        self.store.delete(key, expected).await?;
        Ok(Applied {
            output: (),
            action: AuditAction::SettingDeleted {
                key: key.to_owned(),
                secret: def.kind == Kind::Secret,
            },
        })
    }

    /// Ключ, который консоль вправе менять. `Locked` проверяется раньше `NotPersistent`:
    /// в режиме memory все ключи из окружения, и `setting-locked` говорит оператору больше.
    fn writable(&self, key: &str) -> Result<SettingDef, SettingError> {
        let def = self
            .schema
            .definitions()
            .iter()
            .find(|def| def.key == key && def.console)
            .copied()
            .ok_or(SettingError::Unknown)?;
        if let Some(inherited) = self.schema.inherited(key) {
            return Err(SettingError::Locked { var: inherited.var });
        }
        if !self.persistent {
            return Err(SettingError::NotPersistent);
        }
        Ok(def)
    }

    /// Единственная точка эмиссии аудита: ровно одно событие на запись или удаление.
    /// В отказе нет значения: опечатка в имени ключа не должна унести токен в журнал.
    fn audited<T>(
        &self,
        caller: &Caller,
        key: &str,
        result: Result<Applied<T>, SettingError>,
    ) -> Result<T, SettingError> {
        let (action, outcome) = match &result {
            Ok(applied) => (applied.action.clone(), Outcome::Success),
            Err(error) => (
                AuditAction::SettingRejected {
                    key: key.to_owned(),
                    env_var: error.env_var(),
                },
                // Только Display: цепочка `source` может содержать адреса хранилища.
                Outcome::Failure {
                    reason: error.to_string(),
                    code: Some(error.code()),
                },
            ),
        };
        self.audit.record(AuditEvent {
            actor: Some(caller.principal.clone()),
            request: caller.request.clone(),
            action,
            outcome,
        });
        result.map(|applied| applied.output)
    }
}

/// Что изменилось: прежнее несекретное значение, перезапись нечитаемого секрета и адреса секрета-URL.
fn change_of(previous: Option<&Stored>, value: &NewValue) -> SettingChange {
    match value {
        NewValue::Plain(new) => SettingChange::Plain {
            old: previous.and_then(|row| match &row.value {
                StoredValue::Plain(text) => Some(text.clone()),
                _ => None,
            }),
            new: new.clone(),
        },
        NewValue::Secret(new) => SettingChange::Secret {
            replaced_unreadable: previous
                .is_some_and(|row| matches!(row.value, StoredValue::Unreadable(_))),
            old_locator: previous.and_then(|row| match &row.value {
                StoredValue::Secret(old) => locator(old.expose()),
                _ => None,
            }),
            new_locator: locator(new.expose()),
        },
    }
}
