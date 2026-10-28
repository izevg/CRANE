//! Реализация порта `SettingsSchema` над теми же слоями, из которых строится `Config`.

use std::collections::BTreeMap;
use std::path::Path;

use crane_domain::settings::{Kind, NewValue, SettingDef, Stored};
use crane_ports::settings::{Inherited, Invalid, InvalidKey, SettingsSchema};

use super::stored::{StoredLayer, inherited_var};
use crate::config::{Config, ConfigError, Lookup, ReadFile, definition, definitions};

/// Схема консоли настроек: что за ключи, что их задало на старте и что скажет разбор секции.
/// Окружение и файл не меняются за жизнь процесса, поэтому слой читается через общие замыкания.
pub(crate) struct ConfigSchema {
    inherited: Lookup,
    read_file: ReadFile,
    /// Режим каталога нужен проверке blob; сам `Database` с паролем схеме не передаётся.
    persistent: bool,
}

impl ConfigSchema {
    pub(crate) fn new(inherited: Lookup, read_file: ReadFile, persistent: bool) -> Self {
        Self {
            inherited,
            read_file,
            persistent,
        }
    }
}

impl SettingsSchema for ConfigSchema {
    fn definitions(&self) -> &[SettingDef] {
        definitions()
    }

    fn inherited(&self, key: &str) -> Option<Inherited> {
        let def = definition(key)?;
        let var = inherited_var(def, &|name: &str| (self.inherited)(name))?;
        // Секрет наружу не выходит, ни при `X`, ни при `X_FILE`.
        let value = (def.kind != Kind::Secret)
            .then(|| (self.inherited)(&var))
            .flatten();
        Some(Inherited { var, value })
    }

    fn check(
        &self,
        group: &str,
        stored: &BTreeMap<String, Stored>,
        changes: &[(&str, Option<&NewValue>)],
    ) -> Result<(), Invalid> {
        let inherited = |name: &str| (self.inherited)(name);
        let read_file = |path: &Path| (self.read_file)(path);
        // Строки других секций не участвуют: нечитаемый секрет соседа не должен ронять эту проверку.
        let rows = stored
            .iter()
            .filter(|(key, _)| definition(key).is_some_and(|def| def.group == group));
        let layer = match StoredLayer::new(&inherited, rows, changes) {
            Ok((layer, _ignored)) => layer,
            Err(error) => return Err(invalid(group, vec![error])),
        };
        let overlay = |name: &str| inherited(name).or_else(|| layer.get(name));
        let errors = Config::check_section(group, &overlay, &read_file, self.persistent);
        if errors.is_empty() {
            Ok(())
        } else {
            Err(invalid(group, errors))
        }
    }
}

fn invalid(group: &str, errors: Vec<ConfigError>) -> Invalid {
    Invalid {
        errors: errors
            .into_iter()
            .map(|error| InvalidKey {
                key: key_of(group, &error),
                // Display ошибок конфигурации значений не содержит (см. `ConfigError`).
                detail: error.to_string(),
            })
            .collect(),
    }
}

/// Ключ, к которому относится ошибка: у строки БД он назван прямо, у разбора выводится из переменной.
/// Ошибка, не относящаяся ни к одному ключу секции, записывается на саму секцию.
fn key_of(group: &str, error: &ConfigError) -> String {
    if let ConfigError::Unreadable { key, .. } = error {
        return key.clone();
    }
    error
        .var()
        .and_then(definition_of_var)
        .map_or_else(|| group.to_owned(), |def| def.key.to_owned())
}

/// Определение по переменной, в том числе по `X_FILE` секрета.
fn definition_of_var(var: &str) -> Option<&'static SettingDef> {
    definitions().iter().find(|def| {
        def.env_var == var
            || (def.kind == Kind::Secret && var.strip_suffix("_FILE") == Some(def.env_var))
    })
}

#[cfg(test)]
#[path = "tests/schema.rs"]
mod tests;
