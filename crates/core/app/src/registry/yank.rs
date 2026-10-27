use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::VersionState;
use crane_ports::registry::{BlobStore, Catalog};

use super::{Coordinate, Registry, RegistryError};

impl<C: Catalog, B: BlobStore> Registry<C, B> {
    /// Ставит или снимает yank. Идемпотентно: повторный вызов не ошибка.
    /// Yanked ортогонален состоянию, поэтому допустим для любой версии, кроме tombstone.
    pub async fn set_yanked(
        &self,
        caller: &Caller,
        coordinate: &Coordinate,
        yanked: bool,
    ) -> Result<(), RegistryError> {
        let action = if yanked {
            AuditAction::Yank(coordinate.clone())
        } else {
            AuditAction::Unyank(coordinate.clone())
        };
        let result = self.try_set_yanked(coordinate, yanked).await;
        self.audited(caller, action, &result);
        result
    }

    async fn try_set_yanked(
        &self,
        coordinate: &Coordinate,
        yanked: bool,
    ) -> Result<(), RegistryError> {
        let version = self.find(coordinate).await?;
        if version.state == VersionState::Deleted {
            return Err(RegistryError::Unavailable {
                state: VersionState::Deleted,
            });
        }
        self.catalog.set_yanked(version.id, yanked).await?;
        Ok(())
    }
}
