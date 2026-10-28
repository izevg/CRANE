//! Имена файлов tarball: `{basename}-{version}.tgz`.

use crane_domain::registry::{Format, PackageName, Version};

const EXTENSION: &str = ".tgz";

/// Базовое имя: для `@scope/name` — часть после `/`.
fn basename(package: &PackageName) -> &str {
    let written = package.as_written();
    written.rsplit_once('/').map_or(written, |(_, name)| name)
}

/// Имя файла tarball версии.
#[must_use]
pub fn tarball_file(package: &PackageName, version: &Version) -> String {
    format!("{}-{}{EXTENSION}", basename(package), version.as_written())
}

/// Версия по имени файла; обратная к [`tarball_file`], строгая: любой другой вид файла — `None`.
#[must_use]
pub fn tarball_version(package: &PackageName, file: &str) -> Option<Version> {
    let raw = file
        .strip_suffix(EXTENSION)?
        .strip_prefix(basename(package))?
        .strip_prefix('-')?;
    Version::parse(Format::Npm, raw).ok()
}
