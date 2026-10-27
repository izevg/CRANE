use crane_memory::{MemoryBlobStore, MemoryCatalog};
use crane_ports::contract::{blob_store_contract, catalog_contract};

#[test]
fn memory_catalog_satisfies_catalog_contract() {
    futures_executor::block_on(catalog_contract(MemoryCatalog::default));
}

#[test]
fn memory_blob_store_satisfies_blob_store_contract() {
    futures_executor::block_on(blob_store_contract(MemoryBlobStore::default));
}
