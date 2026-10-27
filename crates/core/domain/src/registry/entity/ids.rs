use std::fmt;

use uuid::Uuid;

/// Идентификатор репозитория.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RepositoryId(Uuid);

/// Идентификатор версии пакета.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VersionId(Uuid);

macro_rules! uuid_id {
    ($name:ident) => {
        impl $name {
            #[must_use]
            pub const fn new(id: Uuid) -> Self {
                Self(id)
            }

            #[must_use]
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

uuid_id!(RepositoryId);
uuid_id!(VersionId);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_keeps_uuid_and_displays_it() {
        // Arrange
        let uuid = Uuid::from_u128(0x1234);

        // Act
        let id = VersionId::new(uuid);

        // Assert
        assert_eq!((id.as_uuid(), id.to_string()), (uuid, uuid.to_string()));
    }
}
