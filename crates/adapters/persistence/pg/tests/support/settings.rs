//! Помощники для тестов настроек и мастера первого запуска.

use std::sync::Arc;
use std::time::SystemTime;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crane_domain::settings::Secret;
use crane_pg::{PgOnboarding, PgSettings, SettingsKey};
use crane_ports::system::Clock;
use sqlx::PgPool;

use super::TestServer;

/// Часы с фиксированным моментом: время записи в тестах не зависит от стенного.
pub(crate) struct FixedClock(pub(crate) SystemTime);

impl Clock for FixedClock {
    fn now(&self) -> SystemTime {
        self.0
    }
}

pub(crate) fn clock() -> Arc<dyn Clock> {
    Arc::new(FixedClock(SystemTime::UNIX_EPOCH))
}

/// Ключ из 32 одинаковых байт `fill`: разные `fill` дают разные ключи и отпечатки.
pub(crate) fn settings_key(fill: u8) -> SettingsKey {
    let encoded = STANDARD.encode([fill; 32]);
    SettingsKey::from_base64(&Secret::new(encoded)).expect("ключ валиден")
}

impl TestServer {
    /// Хранилище настроек на свежей БД; пул возвращается для прямого доступа к таблицам.
    pub(crate) async fn fresh_settings(&self, key: Option<SettingsKey>) -> (PgSettings, PgPool) {
        let pool = self.fresh_pool().await;
        (PgSettings::new(pool.clone(), clock(), key), pool)
    }

    pub(crate) async fn fresh_onboarding(&self, clock: Arc<dyn Clock>) -> PgOnboarding {
        PgOnboarding::new(self.fresh_pool().await, clock)
    }
}
