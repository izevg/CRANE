use super::RepositoryId;
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
