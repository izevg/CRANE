use uuid::Uuid;

/// Идентификатор репозитория.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepositoryId(Uuid);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_keeps_uuid() {
        // Arrange
        let uuid = Uuid::from_u128(0x1234);

        // Act
        let id = VersionId::new(uuid);

        // Assert
        assert_eq!(id.as_uuid(), uuid);
    }
}
