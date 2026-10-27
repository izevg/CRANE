use std::time::Duration;

use bytes::Bytes;
use crane_domain::proxy::ProxyKey;
use crane_ports::proxy::{CacheError, Cached, ProxyCache};
use fred::clients::Pipeline;
use fred::interfaces::{KeysInterface, PubsubInterface};

use super::codec::{decode, encode};
use super::{entry_key, expiration};
use crate::node::{CommandError, INVALIDATION_CHANNEL, Valkey};

fn backend(error: impl std::error::Error + Send + Sync + 'static) -> CacheError {
    CacheError::Backend(Box::new(error))
}

impl ProxyCache for Valkey {
    async fn get(&self, key: &ProxyKey) -> Result<Option<Cached>, CacheError> {
        let node = &self.0;
        let name = entry_key(key);
        if let Some(hit) = node.local.l1.get(&name) {
            return Ok(Some(hit));
        }
        let seen = node.local.generation();
        let raw: Option<Bytes> = node
            .commands
            .get(name.as_str())
            .await
            .map_err(|error| backend(CommandError::from(error)))?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let entry = decode(raw).map_err(backend)?;
        node.local.l1.insert(name.clone(), entry.clone());
        // Пока запись шла из L2 в L1, могла прийти инвалидация: тогда в L1 лежит устаревшее.
        if node.local.generation() != seen {
            node.local.l1.invalidate(&name);
        }
        Ok(Some(entry))
    }

    async fn put(
        &self,
        key: &ProxyKey,
        entry: Cached,
        keep_for: Duration,
    ) -> Result<(), CacheError> {
        let node = &self.0;
        let name = entry_key(key);
        let raw = encode(&entry).map_err(backend)?;
        // L1 заполняется до сети: при недоступном Valkey он остаётся кэшем в процессе.
        // Собственное эхо `inv` выбросит запись из L1 - это одно лишнее чтение L2.
        node.local.l1.insert(name.clone(), entry);
        let pipeline = Pipeline::from(node.commands.clone());
        pipeline
            .set::<(), _, _>(name.as_str(), raw, expiration(keep_for), None, false)
            .await
            .map_err(|error| backend(CommandError::from(error)))?;
        pipeline
            .publish::<(), _, _>(INVALIDATION_CHANNEL, name.as_str())
            .await
            .map_err(|error| backend(CommandError::from(error)))?;
        pipeline
            .all::<()>()
            .await
            .map_err(|error| backend(CommandError::from(error)))
    }
}
