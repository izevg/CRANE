use std::time::SystemTime;

use crane_domain::settings::{Origin, SettingDef, Stored, StoredValue, Unreadable, Version};
use crane_ports::settings::Inherited;

/// Настройка глазами консоли: три факта, остальное выводится. Секрет сюда не попадает по построению.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingView {
    pub def: SettingDef,
    /// Что задало ключ на старте (окружение или файл).
    pub inherited: Option<Inherited>,
    /// Строка хранилища, если она есть.
    pub stored: Option<StoredShown>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredShown {
    pub version: Version,
    pub updated_at: SystemTime,
    pub updated_by: String,
    pub value: Shown,
}

/// `StoredValue` без содержимого секрета.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Shown {
    Plain(String),
    /// Секрет задан; значение наружу не выходит.
    Secret,
    Unreadable(Unreadable),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsList {
    pub settings: Vec<SettingView>,
    /// Сохранённые версии отличаются от снимка, с которым процесс стартовал.
    pub restart_required: bool,
}

impl SettingView {
    /// Откуда взято действующее значение.
    #[must_use]
    pub const fn origin(&self) -> Origin {
        match (&self.inherited, &self.stored) {
            (Some(_), _) => Origin::Env,
            (None, Some(_)) => Origin::Stored,
            (None, None) => Origin::Default,
        }
    }

    /// Консоль ключ менять не может.
    #[must_use]
    pub const fn locked(&self) -> bool {
        self.inherited.is_some() || !self.def.console
    }

    /// Строка в БД есть, но не действует: ключ перекрыт окружением или файлом.
    #[must_use]
    pub const fn shadowed(&self) -> Option<&StoredShown> {
        match (&self.inherited, &self.stored) {
            (Some(_), Some(stored)) => Some(stored),
            _ => None,
        }
    }
}

impl StoredShown {
    pub(super) fn from_row(row: &Stored) -> Self {
        let value = match &row.value {
            StoredValue::Plain(text) => Shown::Plain(text.clone()),
            StoredValue::Secret(_) => Shown::Secret,
            StoredValue::Unreadable(reason) => Shown::Unreadable(reason.clone()),
        };
        Self {
            version: row.version,
            updated_at: row.updated_at,
            updated_by: row.updated_by.clone(),
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use crane_domain::settings::Kind;

    use super::*;

    const DEF: SettingDef = SettingDef {
        key: "blobs",
        env_var: "CRANE_BLOBS",
        group: "blobs",
        kind: Kind::Text,
        unit: None,
        console: true,
    };

    fn inherited() -> Inherited {
        Inherited {
            var: "CRANE_BLOBS".to_owned(),
            value: None,
        }
    }

    fn stored() -> StoredShown {
        StoredShown {
            version: Version::FIRST,
            updated_at: SystemTime::UNIX_EPOCH,
            updated_by: "admin".to_owned(),
            value: Shown::Plain("x".to_owned()),
        }
    }

    fn view(def: SettingDef, inherited: bool, stored_row: bool) -> SettingView {
        SettingView {
            def,
            inherited: inherited.then(self::inherited),
            stored: stored_row.then(stored),
        }
    }

    #[test]
    fn origin_prefers_inherited_over_stored_over_default() {
        // Arrange
        let cases = [(true, true), (true, false), (false, true), (false, false)];

        // Act
        let actual = cases.map(|(env, row)| view(DEF, env, row).origin());

        // Assert
        assert_eq!(
            actual,
            [Origin::Env, Origin::Env, Origin::Stored, Origin::Default]
        );
    }

    #[test]
    fn locked_when_inherited_or_hidden_from_console() {
        // Arrange
        let hidden = SettingDef {
            console: false,
            ..DEF
        };
        let cases = [(DEF, false), (DEF, true), (hidden, false), (hidden, true)];

        // Act
        let actual = cases.map(|(def, env)| view(def, env, false).locked());

        // Assert
        assert_eq!(actual, [false, true, true, true]);
    }

    #[test]
    fn shadowed_only_when_stored_row_is_overridden_by_inherited() {
        // Arrange
        let cases = [(true, true), (true, false), (false, true), (false, false)];

        // Act
        let actual = cases.map(|(env, row)| view(DEF, env, row).shadowed().is_some());

        // Assert
        assert_eq!(actual, [true, false, false, false]);
    }
}
