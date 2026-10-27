//! Bounded context «реестр»: координаты пакетов, дайджесты, жизненный цикл версий.

mod coordinate;
mod digest;
mod entity;
mod lifecycle;

pub use coordinate::{Format, FormatError, NameError, PackageName, Version, VersionError};
pub use digest::{DigestError, Sha256};
pub use entity::{
    Asset, FormatMetadata, PackageVersion, Repository, RepositoryId, RepositoryKind, VersionId,
};
pub use lifecycle::{TransitionError, VersionState};
