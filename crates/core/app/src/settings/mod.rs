//! Use cases настроек консоли (20.1): просмотр, запись и удаление значений с аудитом каждой записи и отказа.

mod error;
mod view;
mod write;

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

use crane_domain::settings::Version;
use crane_ports::audit::AuditSink;
use crane_ports::settings::{SettingsSchema, SettingsStore};

pub use error::SettingError;
pub use view::{SettingView, SettingsList, Shown, StoredShown};

/// Use cases настроек над хранилищем и схемой. Время записи ставит адаптер, время события — sink.
pub struct Settings<S> {
    store: S,
    schema: Arc<dyn SettingsSchema>,
    audit: Arc<dyn AuditSink>,
    /// `false` в режиме memory: записывать некуда, консоль только читает.
    persistent: bool,
    /// Версии строк, с которыми процесс стартовал: сравнением с текущими считается `restart_required`.
    started: BTreeMap<String, Version>,
}

impl<S> fmt::Debug for Settings<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Порты и `dyn`-зависимости не Debug.
        f.debug_struct("Settings").finish_non_exhaustive()
    }
}

impl<S: SettingsStore> Settings<S> {
    #[must_use]
    pub fn new(
        store: S,
        schema: Arc<dyn SettingsSchema>,
        audit: Arc<dyn AuditSink>,
        persistent: bool,
        started: BTreeMap<String, Version>,
    ) -> Self {
        Self {
            store,
            schema,
            audit,
            persistent,
            started,
        }
    }

    /// Ключи консоли с источником, блокировкой и затенением. Чтение, аудита нет.
    ///
    /// # Errors
    /// Ошибки хранилища; строки неизвестных ключей в список не попадают.
    pub async fn view(&self) -> Result<SettingsList, SettingError> {
        let rows = self.store.list().await?;
        let settings: Vec<SettingView> = self
            .schema
            .definitions()
            .iter()
            .filter(|def| def.console)
            .map(|def| SettingView {
                def: *def,
                inherited: self.schema.inherited(def.key),
                stored: rows.get(def.key).map(StoredShown::from_row),
            })
            .collect();
        // Сравниваются только ключи консоли: чужая строка в БД не должна просить перезапуск.
        let current: BTreeMap<String, Version> = settings
            .iter()
            .filter_map(|view| {
                view.stored
                    .as_ref()
                    .map(|row| (view.def.key.to_owned(), row.version))
            })
            .collect();
        Ok(SettingsList {
            restart_required: current != self.started,
            settings,
        })
    }
}
