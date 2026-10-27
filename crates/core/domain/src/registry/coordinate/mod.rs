//! Координаты артефакта: формат, имя пакета, версия.

mod format;
mod name;
mod version;

pub use format::{Format, FormatError};
pub use name::{NameError, PackageName};
pub use version::{Version, VersionError};
