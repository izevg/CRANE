use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{Asset, PackageVersion, RepositoryKind, VersionId, VersionState};
use crane_ports::registry::{BlobStore, Catalog, NewVersion};

use super::{Publish, Registry, RegistryError, Upload};

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
        let result = self.try_publish(request).await;
        self.audited(caller, action, &result);
        result
    }

    async fn try_publish(&self, request: Publish) -> Result<PackageVersion, RegistryError> {
        let Publish {
            coordinate,
            file,
            metadata,
        } = request;
        let repository = self
            .repository(coordinate.format, &coordinate.repository)
            .await?;
        if repository.kind != RepositoryKind::Hosted {
            return Err(RegistryError::NotHosted);
        }

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
            .catalog
            .publish_version(NewVersion {
                id: VersionId::new(self.ids.generate()),
                repository: repository.id,
                package: coordinate.package,
                version: coordinate.version,
                // Hosted без gates в v0.1.0 публикует сразу в Released; quarantine заменит эту строку.
                state: VersionState::Released,
                published_at: self.clock.now(),
                assets: vec![asset],
                metadata,
            })
            .await?;
        Ok(published)
    }

    /// Складывает тело во временную зону, сверяет заявленный клиентом sha256 и только
    /// после совпадения коммитит blob в CAS: неверное содержимое в CAS не попадает.
    async fn ingest(&self, upload: Upload) -> Result<Asset, RegistryError> {
        let staged = self.blobs.stage(upload.body).await?;
        let actual = staged.digests().sha256;
        if let Some(declared) = upload.declared_sha256
            && declared != actual
        {
            return Err(RegistryError::DigestMismatch { declared, actual });
        }
        let digests = self.blobs.commit(staged).await?;
        Ok(Asset {
            path: upload.path,
            sha256: digests.sha256,
            size: digests.size,
        })
    }
}
