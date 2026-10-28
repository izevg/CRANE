use uuid::Uuid;

/// Идентификатор репозитория.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepositoryId(Uuid);

/// Идентификатор workspace: владельца репозиториев и namespace.
///
/// В v0.2.0 workspace один: [`WorkspaceId::DEFAULT`] владеет всеми репозиториями и
/// namespace. Миграция v0.4.0 вставит строку `workspace` с этим id и добавит внешние ключи.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkspaceId(Uuid);

/// Идентификатор версии пакета.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VersionId(Uuid);

impl RepositoryId {
    #[must_use]
    pub const fn new(id: Uuid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl VersionId {
    #[must_use]
    pub const fn new(id: Uuid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl WorkspaceId {
    /// Единственный источник UUID workspace по умолчанию
    /// (`00000000-0000-0000-0000-000000000001`): литерал нигде не повторяется.
    pub const DEFAULT: Self = Self(Uuid::from_u128(1));

    #[must_use]
    pub const fn new(id: Uuid) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn as_uuid(&self) -> Uuid {
        self.0
    }
}
