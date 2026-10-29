//! Тестовые двойники границ: схема (знает, что валидно) и хранилища с недоступными в memory исходами.

#![allow(
    clippy::unused_async_trait_impl,
    reason = "двойники реализуют async-порт без реального ожидания"
)]

use std::collections::BTreeMap;
use std::sync::Arc;

use crane_app::settings::Settings;
use crane_domain::access::Principal;
use crane_domain::settings::{
    Kind, NewValue, Secret, SettingDef, Stored, StoredValue, Unreadable, Version,
};
use crane_memory::{ManualClock, MemoryAudit, MemorySettings};
use crane_ports::settings::{
    Inherited, Invalid, InvalidKey, SettingsError, SettingsSchema, SettingsStore,
};
use futures_executor::block_on;

use crate::common::{fixed_now, principal};

pub(crate) const LIMIT: &str = "limit";
pub(crate) const TOKEN: &str = "token";
pub(crate) const LOCKED: &str = "locked";
pub(crate) const HIDDEN: &str = "hidden";
pub(crate) const REQUIRED: &str = "required";
pub(crate) const LOCKED_VAR: &str = "CRANE_LOCKED";

const fn def(key: &'static str, kind: Kind, console: bool) -> SettingDef {
    SettingDef {
        key,
        env_var: "CRANE_STUB",
        group: "stub",
        kind,
        unit: None,
        console,
    }
}

const DEFS: [SettingDef; 5] = [
    def(LIMIT, Kind::Number, true),
    def(TOKEN, Kind::Secret, true),
    def(LOCKED, Kind::Text, true),
    def(HIDDEN, Kind::Text, false),
    def(REQUIRED, Kind::Text, true),
];

/// Заглушка схемы: пустое значение невалидно, `required` удалять нельзя, `locked` задан окружением.
pub(crate) struct StubSchema;

impl SettingsSchema for StubSchema {
    fn definitions(&self) -> &[SettingDef] {
        &DEFS
    }

    fn inherited(&self, key: &str) -> Option<Inherited> {
        (key == LOCKED).then(|| Inherited {
            var: LOCKED_VAR.to_owned(),
            value: Some("from-env".to_owned()),
            locator: None,
        })
    }

    fn check(
        &self,
        _group: &str,
        _stored: &BTreeMap<String, Stored>,
        changes: &[(&str, Option<&NewValue>)],
    ) -> Result<(), Invalid> {
        let errors: Vec<InvalidKey> = changes
            .iter()
            .filter(|(key, change)| match change {
                Some(NewValue::Plain(text)) => text.is_empty(),
                Some(NewValue::Secret(secret)) => secret.expose().is_empty(),
                None => *key == REQUIRED,
            })
            .map(|(key, _)| InvalidKey {
                key: (*key).to_owned(),
                detail: "значение не принято".to_owned(),
            })
            .collect();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Invalid { errors })
        }
    }
}

pub(crate) fn memory() -> MemorySettings {
    MemorySettings::new(Arc::new(ManualClock::new(fixed_now())))
}

/// Строки, записанные до старта `Settings`: аудит при этом пуст. Секретом считается ключ `token`.
pub(crate) fn seeded(rows: &[(&str, &str)]) -> MemorySettings {
    let store = memory();
    for (key, value) in rows {
        let value = if *key == TOKEN {
            NewValue::Secret(Secret::new((*value).to_owned()))
        } else {
            NewValue::Plain((*value).to_owned())
        };
        block_on(store.put(key, value, None, &principal())).expect("seed");
    }
    store
}

pub(crate) fn version(raw: u64) -> Version {
    Version::new(raw).expect("версия не ноль")
}

pub(crate) fn world<S: SettingsStore>(
    store: S,
    persistent: bool,
    started: &[(&str, u64)],
) -> (Settings<S>, Arc<MemoryAudit>) {
    let audit = Arc::new(MemoryAudit::default());
    let started = started
        .iter()
        .map(|(key, raw)| ((*key).to_owned(), version(*raw)))
        .collect();
    let settings = Settings::new(
        store,
        Arc::new(StubSchema),
        Arc::clone(&audit) as _,
        persistent,
        started,
    );
    (settings, audit)
}

/// Хранилище с секретом `token` версии 3, который не открывается: в memory такую строку не создать.
pub(crate) struct UnreadableStore(pub(crate) Unreadable);

impl SettingsStore for UnreadableStore {
    async fn list(&self) -> Result<BTreeMap<String, Stored>, SettingsError> {
        let row = Stored {
            value: StoredValue::Unreadable(self.0.clone()),
            version: version(3),
            updated_at: fixed_now(),
            updated_by: "admin".to_owned(),
        };
        Ok(BTreeMap::from([(TOKEN.to_owned(), row)]))
    }

    async fn put(
        &self,
        _key: &str,
        _value: NewValue,
        expected: Option<Version>,
        _by: &Principal,
    ) -> Result<Version, SettingsError> {
        expected
            .and_then(Version::next)
            .ok_or(SettingsError::Conflict)
    }

    async fn delete(&self, _key: &str, _expected: Version) -> Result<(), SettingsError> {
        Ok(())
    }
}

/// Хранилище, чьи записи и удаления падают заданной ошибкой.
pub(crate) struct FailingStore(pub(crate) fn() -> SettingsError);

impl SettingsStore for FailingStore {
    async fn list(&self) -> Result<BTreeMap<String, Stored>, SettingsError> {
        Ok(BTreeMap::new())
    }

    async fn put(
        &self,
        _key: &str,
        _value: NewValue,
        _expected: Option<Version>,
        _by: &Principal,
    ) -> Result<Version, SettingsError> {
        Err((self.0)())
    }

    async fn delete(&self, _key: &str, _expected: Version) -> Result<(), SettingsError> {
        Err((self.0)())
    }
}
