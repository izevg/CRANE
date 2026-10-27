//! Выбор кэша proxy и координатора выборок.

use std::io;
use std::path::Path;

use super::{CACHE, CACHE_FILE};
use crate::config::source::secret;
use crate::config::{ConfigError, MEMORY};

/// Выбор кэша proxy и координатора выборок. Без `Debug`: в адресе Valkey бывает пароль.
pub(crate) enum Cache {
    /// Реплики ничего не делят (single-node).
    Memory,
    Valkey(crane_valkey::Endpoint),
}

impl Cache {
    /// Необязателен, в отличие от `Storage`: кэш не источник истины, его потеря не теряет данных,
    /// поэтому молчаливое `memory` по умолчанию безопасно.
    pub(super) fn from_env(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        if var(CACHE).is_none() && var(CACHE_FILE).is_none() {
            return Ok(Self::Memory);
        }
        let value = secret(var, read_file, CACHE, CACHE_FILE)?;
        if value.expose() == MEMORY {
            return Ok(Self::Memory);
        }
        // Текст ошибки адаптера в ConfigError не попадает: он может цитировать адрес.
        match crane_valkey::Endpoint::parse(value.expose()) {
            Ok(endpoint) => Ok(Self::Valkey(endpoint)),
            Err(crane_valkey::EndpointError::Scheme { scheme }) => {
                Err(ConfigError::Unsupported { var: CACHE, scheme })
            }
            Err(crane_valkey::EndpointError::Invalid) => Err(ConfigError::Invalid {
                var: CACHE,
                reason: "ожидалось memory или valkey[s][-cluster|-sentinel]://…",
            }),
        }
    }
}
