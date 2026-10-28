use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{
    Asset, Checksum, Coordinate, FormatMetadata, Namespace, PackageVersion, Sha256,
};
use crane_ports::registry::{BlobStore, Catalog, CatalogError};

use super::super::{Registry, RegistryError, Upload, served};
use super::Refusal;

/// Запрос на прикрепление файла к версии. `PyPI` грузит sdist и каждое колесо отдельным запросом,
/// Maven - pom, jar и classifier-файлы отдельными PUT, и порядок и параллельность не гарантированы.
#[derive(Debug)]
pub struct Attach {
    pub coordinate: Coordinate,
    pub file: Upload,
    /// `Err` - отказ политики формата: действует для каждого файла, в том числе для файла
    /// к существующей версии. `Ok` пишется, только если этот файл создаёт версию:
    /// метаданные версии пишет первый файл.
    pub metadata: Result<FormatMetadata, Refusal>,
}

/// Заявленный клиентом sha256 файла, если он есть среди сумм.
fn declared_sha256(file: &Upload) -> Option<Sha256> {
    file.declared.iter().find_map(|checksum| match checksum {
        Checksum::Sha256(digest) => Some(*digest),
        Checksum::Sha512(_) | Checksum::Sha1(_) | Checksum::Md5(_) => None,
    })
}

/// Что с путём файла в версии.
enum Slot {
    Free,
    /// Тот же путь и тот же sha256: повтор загрузки, запись не нужна.
    Same,
    /// Путь занят другим содержимым.
    Taken,
}

/// Единственное правило идемпотентности прикрепления: путь решает, свободен ли он,
/// а sha256 - повтор это или конфликт.
fn slot(version: &PackageVersion, path: &str, sha256: &Sha256) -> Slot {
    match version.assets.iter().find(|asset| asset.path == path) {
        None => Slot::Free,
        Some(have) if have.content.sha256 == *sha256 => Slot::Same,
        Some(_) => Slot::Taken,
    }
}

/// Версия пропала после `VersionExists`: версии не исчезают, значит, нарушен инвариант.
fn vanished() -> RegistryError {
    RegistryError::Backend("версия пропала после VersionExists".into())
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
        self.audited_claim(caller, action, self.try_attach(request).await)
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
            .create_version(
                &repository,
                &coordinate,
                asset.clone(),
                metadata,
                Vec::new(),
            )
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
                    .ok_or_else(vanished)?;
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
        if let Some(declared) = declared_sha256(&file) {
            match slot(&version, &file.path, &declared) {
                Slot::Same => return Ok(version),
                Slot::Taken => return Err(RegistryError::AssetExists),
                Slot::Free => {}
            }
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
        match slot(&version, &asset.path, &asset.content.sha256) {
            Slot::Same => Ok(version),
            Slot::Taken => Err(RegistryError::AssetExists),
            Slot::Free => match self.catalog.add_asset(version.id, asset.clone()).await {
                Err(CatalogError::AssetExists) => self.settle_lost_race(&version, &asset).await,
                added => Ok(added?),
            },
        }
    }

    /// Параллельный запрос успел занять путь между снимком версии и записью (клиент ретраит
    /// по таймауту): версия перечитывается один раз и путь оценивается тем же правилом.
    async fn settle_lost_race(
        &self,
        version: &PackageVersion,
        asset: &Asset,
    ) -> Result<PackageVersion, RegistryError> {
        let current = self
            .catalog
            .find_version(version.repository, &version.package, &version.version)
            .await?
            .ok_or_else(vanished)?;
        match slot(&current, &asset.path, &asset.content.sha256) {
            Slot::Same => Ok(current),
            Slot::Free | Slot::Taken => Err(RegistryError::AssetExists),
        }
    }
}
