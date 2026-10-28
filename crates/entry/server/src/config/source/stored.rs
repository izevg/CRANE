//! Слой БД под env и файлом. Приоритет действует по КЛЮЧУ, а не по переменной: у секрета две
//! переменные (`CRANE_TOKEN` и `CRANE_TOKEN_FILE`), и `CRANE_TOKEN_FILE` в окружении затеняет
//! сохранённый `token` целиком. Иначе `secret()` увидел бы обе и вернул бы ложный `Conflict`.
//!
//! Слой чистый: отброшенные строки возвращаются данными (`Ignored`), пишет их вызывающий.

use std::collections::{BTreeMap, HashMap};

use crane_domain::settings::{Kind, NewValue, SettingDef, Stored, StoredValue, Unreadable};

use crate::config::{ConfigError, definition};

/// Почему строка БД не действует.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Ignored {
    /// Ключа нет в таблице определений.
    Unknown,
    /// Ключ `console = false`: его задают только окружение или файл.
    EnvOnly,
    /// Ключ задан окружением или файлом; `var` — переменная, которая его задала.
    Shadowed { var: String },
}

/// Значение строки или причина, по которой его нельзя прочитать.
type Candidate = Result<String, Unreadable>;

/// Строки БД, которые действуют: по имени переменной, как ищет `Config`.
pub(crate) struct StoredLayer {
    by_var: HashMap<&'static str, String>,
}

impl StoredLayer {
    pub(crate) fn empty() -> Self {
        Self {
            by_var: HashMap::new(),
        }
    }

    /// `changes`: кандидаты записи (`Some`) и удаления (`None`) поверх строк; на старте пусто.
    /// Нечитаемый секрет, не затенённый окружением, — ошибка: значения нет, подменять его
    /// умолчанием нельзя (fail-closed).
    pub(crate) fn new<'a>(
        inherited: &impl Fn(&str) -> Option<String>,
        rows: impl IntoIterator<Item = (&'a String, &'a Stored)>,
        changes: &[(&str, Option<&NewValue>)],
    ) -> Result<(Self, Vec<(String, Ignored)>), ConfigError> {
        let mut candidates: BTreeMap<String, Candidate> = rows
            .into_iter()
            .map(|(key, row)| (key.clone(), candidate(&row.value)))
            .collect();
        for (key, change) in changes {
            match change {
                Some(value) => candidates.insert((*key).to_owned(), Ok(new_text(value))),
                None => candidates.remove(*key),
            };
        }
        let mut layer = Self::empty();
        let mut ignored = Vec::new();
        for (key, candidate) in candidates {
            match admit(&key, inherited) {
                Err(why) => ignored.push((key, why)),
                Ok(def) => {
                    let text = candidate.map_err(|reason| ConfigError::Unreadable {
                        key,
                        reason: describe(&reason),
                    })?;
                    layer.by_var.insert(def.env_var, text);
                }
            }
        }
        Ok((layer, ignored))
    }

    pub(crate) fn get(&self, var: &str) -> Option<String> {
        self.by_var.get(var).cloned()
    }
}

/// Переменная, задавшая ключ на старте (`X` или, у секрета, `X_FILE`). Один предикат для слоя,
/// схемы и fail-closed. Замыкание отвечает на любое имя: его же читает `AWS_*_FILE`.
pub(crate) fn inherited_var(
    def: &SettingDef,
    var: &impl Fn(&str) -> Option<String>,
) -> Option<String> {
    let file = (def.kind == Kind::Secret).then(|| format!("{}_FILE", def.env_var));
    std::iter::once(def.env_var.to_owned())
        .chain(file)
        .find(|name| var(name).is_some())
}

/// Определение ключа, если строка может действовать.
fn admit(
    key: &str,
    inherited: &impl Fn(&str) -> Option<String>,
) -> Result<&'static SettingDef, Ignored> {
    let def = definition(key).ok_or(Ignored::Unknown)?;
    if !def.console {
        return Err(Ignored::EnvOnly);
    }
    match inherited_var(def, inherited) {
        Some(var) => Err(Ignored::Shadowed { var }),
        None => Ok(def),
    }
}

fn candidate(value: &StoredValue) -> Candidate {
    match value {
        StoredValue::Plain(text) => Ok(text.clone()),
        StoredValue::Secret(secret) => Ok(secret.expose().to_owned()),
        StoredValue::Unreadable(reason) => Err(reason.clone()),
    }
}

fn new_text(value: &NewValue) -> String {
    match value {
        NewValue::Plain(text) => text.clone(),
        NewValue::Secret(secret) => secret.expose().to_owned(),
    }
}

/// Только отпечатки ключей: байтов шифртекста и значений здесь нет.
fn describe(reason: &Unreadable) -> String {
    match reason {
        Unreadable::MissingKey => {
            "задайте CRANE_SETTINGS_KEY: строка запечатана, а ключа нет".to_owned()
        }
        Unreadable::WrongKey {
            stored_key_id,
            current_key_id,
        } => format!("строка запечатана ключом {stored_key_id}, текущий ключ {current_key_id}"),
        Unreadable::Corrupted => "шифртекст повреждён или принадлежит другой строке".to_owned(),
    }
}

#[cfg(test)]
#[path = "tests/stored.rs"]
mod tests;
