//! Bounded context «реестр»: координаты пакетов, дайджесты, жизненный цикл версий.

mod coordinate;
mod digest;
mod ecosystem;
mod entity;
mod lifecycle;
mod locator;
mod namespace;
mod purl;

pub use coordinate::{Format, FormatError, NameError, PackageName, Version, VersionError};
pub use digest::{Checksum, DigestError, Sha1, Sha256, Sha512};
pub use ecosystem::{Ecosystem, Ecosystems, NamespaceShape, OrderKey, OrderKeyBuilder};
pub use entity::{
    Asset, FormatMetadata, PackageVersion, Repository, RepositoryId, RepositoryKind, VersionId,
    WorkspaceId,
};
pub use lifecycle::{DeletePolicy, TransitionError, VersionState};
pub use locator::{Coordinate, PackageRef};
pub use namespace::{Namespace, NamespacePattern, PatternError, hosted_name_allowed};
pub use purl::Purl;
