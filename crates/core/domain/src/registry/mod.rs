//! Bounded context «реестр»: координаты пакетов, дайджесты, жизненный цикл версий.

mod coordinate;
mod digest;
mod entity;
mod lifecycle;
mod locator;
mod namespace;

pub use coordinate::{Format, FormatError, NameError, PackageName, Version, VersionError};
pub use digest::{Checksum, DigestError, Sha1, Sha256, Sha512};
pub use entity::{
    Asset, FormatMetadata, PackageVersion, Repository, RepositoryId, RepositoryKind, VersionId,
    WorkspaceId,
};
pub use lifecycle::{DeletePolicy, TransitionError, VersionState};
pub use locator::{Coordinate, PackageRef};
pub use namespace::{Namespace, NamespacePattern, PatternError, hosted_name_allowed};
