use std::collections::BTreeMap;
use std::fmt;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use crane_domain::access::Principal;
use crane_domain::settings::{Secret, Stored, StoredValue, Version};
use crane_ports::settings::{SettingsError, SettingsStore};
use crane_ports::system::Clock;

/// Настройки в памяти. Не шифрует: секрет лежит как есть, шифр — деталь pg-адаптера.
pub struct MemorySettings {
    clock: Arc<dyn Clock>,
    rows: Mutex<BTreeMap<String, Stored>>,
}

impl fmt::Debug for MemorySettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Строки могут содержать секреты, поэтому содержимое не печатается.
        f.debug_struct("MemorySettings").finish_non_exhaustive()
    }
}

impl MemorySettings {
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>) -> Self {
        Self {
            clock,
            rows: Mutex::new(BTreeMap::new()),
        }
    }

    // Замок берётся только в синхронных функциях: guard не должен жить через await.
    fn lock(&self) -> MutexGuard<'_, BTreeMap<String, Stored>> {
        self.rows.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn put_sync(
        &self,
        key: &str,
        value: StoredValue,
        expected: Option<Version>,
        by: &Principal,
    ) -> Result<Version, SettingsError> {
        let mut rows = self.lock();
        let version = match (rows.get(key), expected) {
            (None, None) => Version::FIRST,
            (Some(current), Some(expected)) if current.version == expected => {
                expected.next().ok_or_else(|| {
                    SettingsError::Backend(Box::new(io::Error::other(
                        "версия настройки переполнена",
                    )))
                })?
            }
            _ => return Err(SettingsError::Conflict),
        };
        rows.insert(
            key.to_owned(),
            Stored {
                value,
                version,
                updated_at: self.clock.now(),
                updated_by: by.id.clone(),
            },
        );
        Ok(version)
    }

    fn delete_sync(&self, key: &str, expected: Version) -> Result<(), SettingsError> {
        let mut rows = self.lock();
        if rows.get(key).is_some_and(|row| row.version == expected) {
            rows.remove(key);
            Ok(())
        } else {
            Err(SettingsError::Conflict)
        }
    }
}

/// `Secret` не `Clone` намеренно, поэтому копия строится явно и только здесь.
fn duplicate(stored: &Stored) -> Stored {
    let value = match &stored.value {
        StoredValue::Plain(text) => StoredValue::Plain(text.clone()),
        StoredValue::Secret(secret) => StoredValue::Secret(Secret::new(secret.expose().to_owned())),
    };
    Stored {
        value,
        version: stored.version,
        updated_at: stored.updated_at,
        updated_by: stored.updated_by.clone(),
    }
}

impl SettingsStore for MemorySettings {
    fn list(&self) -> impl Future<Output = Result<BTreeMap<String, Stored>, SettingsError>> + Send {
        std::future::ready(Ok(self
            .lock()
            .iter()
            .map(|(key, stored)| (key.clone(), duplicate(stored)))
            .collect()))
    }

    fn put(
        &self,
        key: &str,
        value: StoredValue,
        expected: Option<Version>,
        by: &Principal,
    ) -> impl Future<Output = Result<Version, SettingsError>> + Send {
        std::future::ready(self.put_sync(key, value, expected, by))
    }

    fn delete(
        &self,
        key: &str,
        expected: Version,
    ) -> impl Future<Output = Result<(), SettingsError>> + Send {
        std::future::ready(self.delete_sync(key, expected))
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime};

    use crane_domain::access::PrincipalKind;

    use super::*;
    use crate::ManualClock;

    #[test]
    fn put_stamps_row_with_clock_time() {
        // Arrange
        let start = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
        let clock = Arc::new(ManualClock::new(start));
        let store = MemorySettings::new(clock.clone());
        let by = Principal {
            id: "alice".to_owned(),
            kind: PrincipalKind::Human,
        };
        clock.advance(Duration::from_secs(7));

        // Act
        let put = futures_executor::block_on(store.put(
            "blobs",
            StoredValue::Plain("x".to_owned()),
            None,
            &by,
        ));

        // Assert
        assert!(put.is_ok());
        let rows = futures_executor::block_on(store.list());
        let stamps: Vec<SystemTime> = rows
            .map(|rows| rows.into_values().map(|row| row.updated_at).collect())
            .unwrap_or_default();
        assert_eq!(stamps, [start + Duration::from_secs(7)]);
    }
}
