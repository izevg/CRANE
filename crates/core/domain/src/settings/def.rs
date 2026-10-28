/// Метаданные настройки. Сами значения здесь не хранятся: таблицы `SETTINGS` лежат рядом
/// с `VARS` каждой секции конфигурации.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettingDef {
    pub key: &'static str,
    pub env_var: &'static str,
    /// Секция конфигурации, она же группа экрана и шаг мастера.
    pub group: &'static str,
    pub kind: Kind,
    pub unit: Option<Unit>,
    /// Показывать ли ключ в консоли.
    pub console: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Text,
    Number,
    Bool,
    List,
    Url,
    Secret,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unit {
    Bytes,
    Seconds,
}

/// Откуда взято действующее значение.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    Env,
    Stored,
    Default,
}
