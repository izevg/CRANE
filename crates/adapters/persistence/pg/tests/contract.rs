//! Контракт порта `Catalog` против настоящего PostgreSQL.

#[allow(
    dead_code,
    reason = "помощники общие для нескольких тестовых бинарников"
)]
mod support;

use crane_ports::contract::{
    catalog_contract, namespace_contract, onboarding_store_contract, settings_store_contract,
};
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

#[test]
fn pg_catalog_satisfies_namespace_contract() {
    support::run(async |server| {
        let handle = Handle::current();

        namespace_contract(|| block_in_place(|| handle.block_on(server.fresh_catalog()))).await;
    });
}

#[test]
fn pg_settings_satisfies_settings_store_contract() {
    support::run(async |server| {
        let handle = Handle::current();

        settings_store_contract(|| {
            block_in_place(|| {
                handle.block_on(server.fresh_settings(Some(support::settings::settings_key(1))))
            })
            .0
        })
        .await;
    });
}

#[test]
fn pg_onboarding_satisfies_onboarding_store_contract() {
    support::run(async |server| {
        let handle = Handle::current();

        onboarding_store_contract(|clock| {
            block_in_place(|| handle.block_on(server.fresh_onboarding(clock)))
        })
        .await;
    });
}
