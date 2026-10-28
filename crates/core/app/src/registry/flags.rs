use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{PackageRef, VersionState};
use crane_ports::registry::{BlobStore, Catalog};

use super::{Coordinate, Registry, RegistryError, served};

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

    /// Ставит (`Some`) или снимает (`None`) сообщение об устаревании. Идемпотентно.
    /// Устаревание ортогонально состоянию, как yank: допустимо для любой версии, кроме tombstone.
    pub async fn set_deprecated(
        &self,
        caller: &Caller,
        coordinate: &Coordinate,
        message: Option<String>,
    ) -> Result<(), RegistryError> {
        let action = AuditAction::Deprecate {
            coordinate: coordinate.clone(),
            message: message.clone(),
        };
        let result = self.try_set_deprecated(coordinate, message).await;
        self.audited(caller, action, &result);
        result
    }

    async fn try_set_deprecated(
        &self,
        coordinate: &Coordinate,
        message: Option<String>,
    ) -> Result<(), RegistryError> {
        let version = self.find(coordinate).await?;
        if version.state == VersionState::Deleted {
            return Err(RegistryError::Unavailable {
                state: VersionState::Deleted,
            });
        }
        self.catalog.set_deprecated(version.id, message).await?;
        Ok(())
    }

    /// Ставит dist-тег на версию (переносит, если тег уже стоит на другой). Идемпотентно.
    /// Тег указывает только на выданную клиентам версию (`Released`).
    pub async fn set_tag(
        &self,
        caller: &Caller,
        coordinate: &Coordinate,
        tag: &str,
    ) -> Result<(), RegistryError> {
        let action = AuditAction::Tag {
            coordinate: coordinate.clone(),
            tag: tag.to_owned(),
        };
        let result = self.try_set_tag(coordinate, tag).await;
        self.audited(caller, action, &result);
        result
    }

    async fn try_set_tag(&self, coordinate: &Coordinate, tag: &str) -> Result<(), RegistryError> {
        let version = self.find(coordinate).await?;
        served(&version)?;
        self.catalog.set_tag(version.id, tag).await?;
        Ok(())
    }

    /// Снимает тег. Отсутствующий тег: `TagNotFound`.
    pub async fn remove_tag(
        &self,
        caller: &Caller,
        package: &PackageRef,
        tag: &str,
    ) -> Result<(), RegistryError> {
        let action = AuditAction::Untag {
            package: package.clone(),
            tag: tag.to_owned(),
        };
        let result = self.try_remove_tag(package, tag).await;
        self.audited(caller, action, &result);
        result
    }

    async fn try_remove_tag(&self, package: &PackageRef, tag: &str) -> Result<(), RegistryError> {
        let repository = self
            .find_repository(package.format, &package.repository)
            .await?;
        if self
            .catalog
            .remove_tag(repository.id, &package.package, tag)
            .await?
        {
            Ok(())
        } else {
            Err(RegistryError::TagNotFound)
        }
    }
}
