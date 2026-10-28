use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{
    Asset, Checksum, Coordinate, FormatMetadata, Namespace, PackageVersion, Sha256, VersionId,
    VersionState,
};
use crane_ports::registry::{BlobStore, Catalog, CatalogError, NewVersion};

use super::super::{Registry, RegistryError, Upload, served};
use super::Refusal;

/// Запрос на прикрепление файла к версии. `PyPI` грузит sdist и каждое колесо отдельным запросом,
/// Maven - pom, jar и classifier-файлы отдельными PUT, и порядок и параллельность не гарантированы.
#[derive(Debug)]
pub struct Attach {
    pub coordinate: Coordinate,
    pub file: Upload,
    /// Используется, только если этот файл создаёт версию: метаданные версии пишет первый файл.
    pub metadata: Result<FormatMetadata, Refusal>,
}

/// Заявленный клиентом sha256 файла, если он есть среди сумм.
fn declared_sha256(file: &Upload) -> Option<Sha256> {
    file.declared.iter().find_map(|checksum| match checksum {
        Checksum::Sha256(digest) => Some(*digest),
        Checksum::Sha512(_) | Checksum::Sha1(_) => None,
    })
}

/// Файл версии с этим путём, если он уже есть.
fn existing<'a>(version: &'a PackageVersion, path: &str) -> Option<&'a Asset> {
    version.assets.iter().find(|asset| asset.path == path)
}

impl<C: Catalog, B: BlobStore> Registry<C, B> {
    /// Прикрепляет файл к версии; первый файл создаёт версию, поэтому отдельного шага
    /// «создать версию» у клиента нет. Ровно одно событие аудита с путём файла, плюс `Reserve`,
    /// если файл захватил новое имя.
    ///
    /// Повтор идемпотентен: тот же путь с тем же sha256 - успех без записи, другой sha256 -
    /// `AssetExists`. Сравнение идёт по заявленному клиентом sha256 до чтения тела (отказ без
    /// blob-«сироты»); если клиент его не заявил, тело читается, а сравнивается посчитанный sha256
    /// (blob - CAS, повторная запись идемпотентна, а сирота при отказе уходит в GC).
    ///
    /// Состояние версии проверяет `served`: к `Quarantined`, `Blocked` и `Deleted` файлы
    /// не прикрепляются. Имя файла уникально навсегда без отдельного правила: ключ
    /// `(версия, путь)` и tombstone версии (файлы `Deleted` остаются) дают это по построению.
    pub async fn attach(
        &self,
        caller: &Caller,
        request: Attach,
    ) -> Result<PackageVersion, RegistryError> {
        let action = AuditAction::PublishFile {
            coordinate: request.coordinate.clone(),
            path: request.file.path.clone(),
        };
        let (result, claim) = match self.try_attach(request).await {
            Ok((attached, claim)) => (Ok(attached), claim),
            Err(error) => (Err(error), None),
        };
        self.audited(caller, action, &result);
        // Как у `publish`: захват имени - отдельный факт безопасности.
        if let Some(claim) = claim {
            self.audited(caller, AuditAction::Reserve(claim), &result);
        }
        result
    }

    async fn try_attach(
        &self,
        request: Attach,
    ) -> Result<(PackageVersion, Option<Namespace>), RegistryError> {
        let Attach {
            coordinate,
            file,
            metadata,
        } = request;
        let repository = self.hosted_repository(&coordinate).await?;
        let metadata = Self::accepted(metadata)?;

        let found = self
            .catalog
            .find_version(repository.id, &coordinate.package, &coordinate.version)
            .await?;
        if let Some(version) = found {
            return Ok((self.attach_to(version, file).await?, None));
        }

        let claim = self
            .admit_publication(&repository, coordinate.format, &coordinate.package)
            .await?;
        let asset = self.ingest(file).await?;
        let created = self
            .catalog
            .publish_version(NewVersion {
                id: VersionId::new(self.ids.generate()),
                repository: repository.id,
                package: coordinate.package.clone(),
                version: coordinate.version.clone(),
                state: VersionState::Released,
                published_at: self.clock.now(),
                assets: vec![asset.clone()],
                metadata,
                tags: Vec::new(),
            })
            .await;
        match created {
            Ok(version) => Ok((version, claim)),
            // Гонка двух первых загрузок: версию создал другой запрос. Повтор один и без цикла:
            // тело уже в CAS и повторно не читается, а версии не исчезают.
            Err(CatalogError::VersionExists) => {
                let winner = self
                    .catalog
                    .find_version(repository.id, &coordinate.package, &coordinate.version)
                    .await?
                    .ok_or(RegistryError::VersionExists)?;
                served(&winner)?;
                Ok((self.place(winner, asset).await?, None))
            }
            Err(error) => Err(error.into()),
        }
    }

    /// Файл к уже существующей версии: сначала правила, потом чтение тела.
    async fn attach_to(
        &self,
        version: PackageVersion,
        file: Upload,
    ) -> Result<PackageVersion, RegistryError> {
        served(&version)?;
        if let (Some(have), Some(declared)) =
            (existing(&version, &file.path), declared_sha256(&file))
        {
            return if have.sha256 == declared {
                Ok(version)
            } else {
                Err(RegistryError::AssetExists)
            };
        }
        let asset = self.ingest(file).await?;
        self.place(version, asset).await
    }

    /// Кладёт уже сохранённый в CAS файл в версию: повтор того же файла - успех без записи.
    async fn place(
        &self,
        version: PackageVersion,
        asset: Asset,
    ) -> Result<PackageVersion, RegistryError> {
        match existing(&version, &asset.path) {
            Some(have) if have.sha256 == asset.sha256 => Ok(version),
            Some(_) => Err(RegistryError::AssetExists),
            None => Ok(self.catalog.add_asset(version.id, asset).await?),
        }
    }
}
