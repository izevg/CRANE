//! Протокол npm-реестра: чистый sans-IO код без HTTP, async и зависимости от `app`/`ports`.
//!
//! `serde`-derive допустим только на границе проводного формата (formats, entry), не в ядре.

mod packument;
mod proxy;
mod reply;
mod route;
mod sri;
mod write;

pub use packument::{DocError, TarballBase, View, content_type, render_hosted, render_upstream};
pub use proxy::{
    ACCEPT_SHA1_ONLY, PACKUMENT, REGISTRY_HOST, Tarball, TarballError, UNPUBLISH, packument_path,
    tarball,
};
pub use reply::{render_error, render_ok, render_whoami};
pub use route::{Route, RouteError, tarball_file, tarball_version};
pub use write::{
    NpmPublish, Write, WriteError, classify, parse_tag, parse_tag_body, removable, render_tags,
    valid_for_new,
};
