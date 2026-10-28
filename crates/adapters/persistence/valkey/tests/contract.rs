//! Контракты портов `ProxyCache` и `FetchLeases` против настоящего Valkey.

#[allow(
    dead_code,
    reason = "помощники общие для нескольких тестовых бинарников"
)]
mod support;

use crane_ports::contract::{fetch_leases_contract, proxy_cache_contract};
use tokio::runtime::Handle;
use tokio::task::block_in_place;

// Каждому тесту своя БД Valkey: тестовые бинарники и функции идут параллельно.
const CACHE_DB: u8 = 1;
const LEASES_DB: u8 = 2;

#[test]
fn valkey_satisfies_proxy_cache_contract() {
    support::run(async |server| {
        // Фабрика контракта синхронна, а подготовка БД асинхронна: блокируем поток
        // рабочего пула, не останавливая сам runtime.
        let handle = Handle::current();

        proxy_cache_contract(|| block_in_place(|| handle.block_on(server.fresh(CACHE_DB)))).await;
    });
}

#[test]
fn valkey_satisfies_fetch_leases_contract() {
    support::run(async |server| {
        let handle = Handle::current();

        fetch_leases_contract(|| block_in_place(|| handle.block_on(server.fresh(LEASES_DB)))).await;
    });
}
