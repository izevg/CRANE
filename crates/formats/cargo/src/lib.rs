//! Протокол Cargo sparse-реестра: чистый sans-IO код без HTTP, async и зависимости от `app`/`ports`.
//!
//! `serde`-derive допустим только на границе проводного формата (formats, entry), не в ядре.

mod api;
mod index;
mod proxy;
mod publish;

pub use api::{
    asset_path, render_config, render_errors, render_ok, render_owners, render_publish_ok,
};
pub use index::{IndexError, index_path, parse_index_path, render_index};
pub use proxy::{Archive, DL_HOST, INDEX, INDEX_HOST, archive};
pub use publish::{CargoPublish, PublishError, parse_publish};
