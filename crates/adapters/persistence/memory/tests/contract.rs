use std::sync::Arc;
use std::time::{Duration, SystemTime};

use crane_memory::{
    ManualClock, MemoryBlobStore, MemoryCatalog, MemoryFetchLeases, MemoryProxyCache,
};
use crane_ports::contract::{
    blob_store_contract, catalog_contract, fetch_leases_contract, namespace_contract,
    proxy_cache_contract,
};

#[test]
fn memory_catalog_satisfies_catalog_contract() {
    futures_executor::block_on(catalog_contract(MemoryCatalog::default));
}

#[test]
fn memory_catalog_satisfies_namespace_contract() {
    futures_executor::block_on(namespace_contract(MemoryCatalog::default));
}

#[test]
fn memory_blob_store_satisfies_blob_store_contract() {
    futures_executor::block_on(blob_store_contract(MemoryBlobStore::default));
}

#[test]
fn memory_proxy_cache_satisfies_proxy_cache_contract() {
    futures_executor::block_on(proxy_cache_contract(MemoryProxyCache::default));
}

// Сценарии контракта не двигают часы: каждому хватает свежего координатора с остановленным временем.
fn fresh_leases() -> MemoryFetchLeases {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000);
    MemoryFetchLeases::new(Arc::new(ManualClock::new(now)))
}

#[test]
fn memory_fetch_leases_satisfy_fetch_leases_contract() {
    futures_executor::block_on(fetch_leases_contract(fresh_leases));
}
