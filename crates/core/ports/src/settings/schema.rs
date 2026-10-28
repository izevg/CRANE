use crane_domain::settings::SettingDef;

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

/// Инверсия: `app` пишет и аудирует, `entry` знает, что валидно. Реализация одна, в `entry/server`.
pub trait SettingsSchema: Send + Sync {
    fn definitions(&self) -> &[SettingDef];

    /// Переменная окружения или файл, задавшие ключ на старте; `Some` — ключ заблокирован.
    fn inherited(&self, key: &str) -> Option<&'static str>;

    /// Прогон секции `group` над наложением кандидата на текущие слои.
    ///
    /// # Errors
    /// Список ключей секции, которые не прошли разбор.
    fn check(&self, group: &str, overlay: &dyn Fn(&str) -> Option<String>) -> Result<(), Invalid>;
}
