mod common;

use std::cell::Cell;

use common::{TempDir, run, store_in};
use crane_ports::contract::blob_store_contract;

#[test]
fn object_blob_store_on_local_fs_satisfies_blob_store_contract() {
    // Arrange: свежий каталог на каждый сценарий контракта
    let root = TempDir::new();
    let next = Cell::new(0u32);

    // Act + Assert: утверждения внутри контракта
    run(blob_store_contract(|| {
        next.set(next.get() + 1);
        store_in(&root.path().join(next.get().to_string()))
    }));
}
