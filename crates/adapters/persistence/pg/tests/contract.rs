//! Контракт порта `Catalog` против настоящего PostgreSQL.

#[allow(
    dead_code,
    reason = "помощники общие для нескольких тестовых бинарников"
)]
mod support;

use crane_ports::contract::catalog_contract;
use tokio::runtime::Handle;
use tokio::task::block_in_place;

#[test]
fn pg_catalog_satisfies_catalog_contract() {
    support::run(async |server| {
        // Фабрика контракта синхронна, а свежая БД создаётся асинхронно: блокируем поток
        // рабочего пула, не останавливая сам runtime.
        let handle = Handle::current();

        catalog_contract(|| block_in_place(|| handle.block_on(server.fresh_catalog()))).await;
    });
}
