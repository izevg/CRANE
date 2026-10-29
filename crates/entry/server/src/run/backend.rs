//! Хранилище настроек процесса: PostgreSQL или память, один тип для `Settings`.

use std::collections::BTreeMap;

use crane_domain::access::Principal;
use crane_domain::settings::{NewValue, Stored, Version};
use crane_memory::MemorySettings;
use crane_pg::PgSettings;
use crane_ports::settings::{SettingsError, SettingsStore};

/// Хранилище настроек: каталог в PostgreSQL или пустое в памяти.
pub(super) enum SettingsBackend {
    Memory(MemorySettings),
    Postgres(PgSettings),
}

impl SettingsStore for SettingsBackend {
    async fn list(&self) -> Result<BTreeMap<String, Stored>, SettingsError> {
        match self {
            Self::Memory(store) => store.list().await,
            Self::Postgres(store) => store.list().await,
        }
    }

    async fn put(
        &self,
        key: &str,
        value: NewValue,
        expected: Option<Version>,
        by: &Principal,
    ) -> Result<Version, SettingsError> {
        match self {
            Self::Memory(store) => store.put(key, value, expected, by).await,
            Self::Postgres(store) => store.put(key, value, expected, by).await,
        }
    }

    async fn delete(&self, key: &str, expected: Version) -> Result<(), SettingsError> {
        match self {
            Self::Memory(store) => store.delete(key, expected).await,
            Self::Postgres(store) => store.delete(key, expected).await,
        }
    }
}
