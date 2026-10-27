//! Content-addressable `BlobStore` поверх `object_store`: локальная ФС и S3 через один адаптер.
//!
//! Раскладка (docs/product/09): blob лежит в `blobs/sha256/<hex[0..2]>/<hex[2..4]>/<hex>`,
//! временные объекты — в `staging/<uuid>`. `open` читает только `blobs/`, поэтому
//! staged-объекты снаружи не видны.

mod config;
mod store;

pub use store::ObjectBlobStore;
