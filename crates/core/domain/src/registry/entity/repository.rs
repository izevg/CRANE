use super::{RepositoryId, WorkspaceId};
use crate::registry::Format;

/// Тип репозитория: свой или прокси к внешнему.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RepositoryKind {
    Hosted,
    Proxy,
}

/// Репозиторий пакетов одного формата.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repository {
    pub id: RepositoryId,
    pub format: Format,
    pub kind: RepositoryKind,
    pub name: String,
}

impl Repository {
    /// Владелец репозитория.
    // Крючок: до `repository.workspace_id` (v0.4.0) все репозитории принадлежат workspace
    // по умолчанию; потребители уже спрашивают владельца у репозитория, а не берут константу.
    #[must_use]
    pub const fn workspace(&self) -> WorkspaceId {
        WorkspaceId::DEFAULT
    }
}
