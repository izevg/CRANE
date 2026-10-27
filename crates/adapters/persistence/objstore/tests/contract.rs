mod common;

use std::cell::Cell;

use common::{TempDir, run, store_in};
use crane_objstore::ObjectBlobStore;
use crane_ports::contract::blob_store_contract;

/// Фабрика свежих хранилищ: каждый сценарий контракта получает свой каталог внутри
/// временного корня, который живёт, пока жива фабрика.
fn fresh_stores() -> impl Fn() -> ObjectBlobStore {
    let root = TempDir::new();
    let next = Cell::new(0u32);
    move || {
        next.set(next.get() + 1);
        store_in(&root.path().join(next.get().to_string()))
    }
}

#[test]
fn object_blob_store_on_local_fs_satisfies_blob_store_contract() {
    run(blob_store_contract(fresh_stores()));
}
