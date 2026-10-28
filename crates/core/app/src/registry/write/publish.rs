use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{Namespace, PackageVersion};
use crane_ports::registry::{BlobStore, Catalog};

use super::super::{Publish, Registry, RegistryError};

impl<C: Catalog, B: BlobStore> Registry<C, B> {
    /// Публикует версию атомарно: blob становится видимым в CAS раньше записи каталога,
    /// а запись каталога — точка фиксации. Записи, указывающей на незакоммиченный blob,
    /// не бывает ни при каком порядке сбоев.
    pub async fn publish(
        &self,
        caller: &Caller,
        request: Publish,
    ) -> Result<PackageVersion, RegistryError> {
        let action = AuditAction::Publish(request.coordinate.clone());
        self.audited_claim(caller, action, self.try_publish(request).await)
    }

    /// Кроме версии отдаёт namespace точного имени, если публикация захватила новое имя.
    async fn try_publish(
        &self,
        request: Publish,
    ) -> Result<(PackageVersion, Option<Namespace>), RegistryError> {
        let Publish {
            coordinate,
            file,
            metadata,
            tags,
        } = request;
        let repository = self.hosted_repository(&coordinate).await?;
        let metadata = Self::accepted(metadata)?;

        let claim = self
            .admit_publication(&repository, coordinate.format, &coordinate.package)
            .await?;

        // Отказ до чтения тела и без blob-«сироты» в CAS; гонку после предпроверки
        // ловит `publish_version` каталога.
        if self
            .catalog
            .find_version(repository.id, &coordinate.package, &coordinate.version)
            .await?
            .is_some()
        {
            return Err(RegistryError::VersionExists);
        }

        let asset = self.ingest(file).await?;

        let published = self
            .create_version(&repository, &coordinate, asset, metadata, tags)
            .await?;
        Ok((published, claim))
    }
}
