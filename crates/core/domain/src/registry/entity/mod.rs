//! Сущности реестра.

mod ids;
mod repository;
mod version;

pub use ids::{RepositoryId, VersionId};
pub use repository::{Repository, RepositoryKind};
pub use version::{Asset, FormatMetadata, PackageVersion};
