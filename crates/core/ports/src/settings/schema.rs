use std::collections::BTreeMap;

use crane_domain::settings::{NewValue, SettingDef, Stored};

/// Ключи, не прошедшие проверку секции.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid {
    pub errors: Vec<InvalidKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidKey {
    pub key: String,
    pub detail: String,
}

/// Значение, заданное окружением или файлом на старте процесса.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inherited {
    /// Переменная окружения (или `X_FILE`), задавшая ключ.
    pub var: String,
    /// `None` у секрета: наружу он не выходит. Это правило держит реализация схемы.
    pub value: Option<String>,
    /// Адрес секрета-URL без учётных данных (`domain::settings::locator`); `None` у несекрета и у
    /// секрета, не похожего на URL.
    pub locator: Option<String>,
}

/// Инверсия: `app` пишет и аудирует, `entry` знает, что валидно. Реализация одна, в `entry/server`.
pub trait SettingsSchema: Send + Sync {
    fn definitions(&self) -> &[SettingDef];

    /// Что задало ключ на старте; `Some` — ключ заблокирован для записи из консоли.
    fn inherited(&self, key: &str) -> Option<Inherited>;

    /// Прогон секции `group` над слоями env > файл > (строки `stored` с применёнными `changes`).
    /// В `changes` `Some(value)` — запись ключа, `None` — удаление. Нечитаемый секрет внутри
    /// секции даёт `InvalidKey` этого ключа, а не ложное «не задан».
    ///
    /// # Errors
    /// Ключи секции, которые не прошли разбор; `errors` в `Err` никогда не пуст.
    fn check(
        &self,
        group: &str,
        stored: &BTreeMap<String, Stored>,
        changes: &[(&str, Option<&NewValue>)],
    ) -> Result<(), Invalid>;
}
