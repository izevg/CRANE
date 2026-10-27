use crane_domain::registry::VersionState;
use crane_ports::registry::{BlobStore, Catalog};

use super::{Coordinate, Registry, RegistryError};

impl<C: Catalog, B: BlobStore> Registry<C, B> {
    /// Ставит или снимает yank. Идемпотентно: повторный вызов не ошибка.
    /// Yanked ортогонален состоянию, поэтому допустим для любой версии, кроме tombstone.
    pub async fn set_yanked(
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
