//! Предметная область CRANE: сущности, state machines, ошибки.
//!
//! Слой не зависит ни от одного другого крейта проекта и не выполняет IO.

pub mod registry;

pub use registry::{
    Asset, DigestError, Format, FormatError, FormatMetadata, NameError, PackageName,
    PackageVersion, Repository, RepositoryId, RepositoryKind, Sha256, TransitionError, Version,
    VersionError, VersionId, VersionState,
};
