//! Протокол PyPI-индекса: чистый sans-IO код без HTTP, async и зависимости от `app`/`ports`.
//!
//! `serde`-derive допустим только на границе проводного формата (formats, entry), не в ядре.

mod ecosystem;
mod simple;
mod upload;

pub use ecosystem::{PYPI, PyPi};
pub use simple::{DistFile, Project, View, Yanked, content_type, hosted, render};
pub use upload::{Accepted, UploadError, admit};
