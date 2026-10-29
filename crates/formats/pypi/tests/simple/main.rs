//! Страница проекта Simple API: выбор представления, HTML, JSON и сборка из каталога.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

#[path = "../common/mod.rs"]
mod common;
mod hosted;
mod html;
mod json;
mod view;

use crane_format_pypi::{DistFile, Project, Yanked};

use crate::common::{digest, pkg};

pub(crate) fn dist(filename: &str) -> DistFile {
    DistFile {
        filename: filename.to_owned(),
        url: format!("https://crane.example/pypi/internal/files/acme/1/{filename}"),
        sha256: digest(0xab),
        requires_python: None,
        yanked: Yanked::No,
        size: 1234,
        upload_time: None,
    }
}

pub(crate) fn project(files: Vec<DistFile>) -> Project {
    Project {
        name: pkg("Acme_Core"),
        versions: vec!["1".to_owned()],
        files,
    }
}
