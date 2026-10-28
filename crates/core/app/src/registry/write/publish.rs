use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{Namespace, PackageVersion, VersionId, VersionState};
use crane_ports::registry::{BlobStore, Catalog, NewVersion};

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
        let (result, claim) = match self.try_publish(request).await {
            Ok((published, claim)) => (Ok(published), claim),
            Err(error) => (Err(error), None),
        };
        self.audited(caller, action, &result);
        // Отступление от «одно событие на use case»: захват имени - отдельный факт
        // безопасности (с него имя становится внутренним), и аудитору он нужен как
        // самостоятельная запись, а не как догадка по первой публикации.
        if let Some(claim) = claim {
            self.audited(caller, AuditAction::Reserve(claim), &result);
        }
        result
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
                tags,
            })
            .await?;
        Ok((published, claim))
    }
}
