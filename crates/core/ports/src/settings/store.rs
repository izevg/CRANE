use crane_domain::access::Principal;
use crane_domain::settings::{Stored, StoredValue, Version};

#[derive(Debug, thiserror::Error)]
pub enum SettingsError {
    /// Версия не совпала, строка уже есть при `expected = None` или строки нет при `Some`.
    #[error("конфликт версии настройки")]
    Conflict,
    /// Адаптер не может хранить секреты: нет ключа шифрования.
    #[error("хранение секретов недоступно: нет ключа шифрования")]
    SecretsUnavailable,
    #[error("ошибка хранилища настроек")]
    Backend(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// Хранилище значений, заданных в консоли. Журнал аудита пишет use case, не адаптер.
///
/// Отказ записи в режиме memory (нет БД) — решение `app`/`entry`, а не порта: иначе
/// memory-адаптер нельзя было бы прогнать контрактом.
pub trait SettingsStore: Send + Sync {
    /// Все строки по ключу по возрастанию.
    fn list(&self) -> impl Future<Output = Result<Vec<(String, Stored)>, SettingsError>> + Send;

    /// Запись с проверкой версии (CAS). `None` — строки быть не должно, новая строка получает
    /// `Version::FIRST`; иначе `expected` обязана совпасть с текущей, результат — следующая версия.
    /// `by` попадает в `updated_by`, время берёт адаптер из `Clock`.
    fn put(
        &self,
        key: &str,
        value: StoredValue,
        expected: Option<Version>,
        by: &Principal,
    ) -> impl Future<Output = Result<Version, SettingsError>> + Send;

    /// Удаляет строку, если её версия равна `expected`.
    fn delete(
        &self,
        key: &str,
        expected: Version,
    ) -> impl Future<Output = Result<(), SettingsError>> + Send;
}
