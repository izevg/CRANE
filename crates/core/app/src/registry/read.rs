use crane_domain::access::Caller;
use crane_domain::registry::{Format, PackageName, PackageVersion};
use crane_ports::registry::{BlobStore, Catalog};

use super::{Coordinate, Download, PackageView, Registry, RegistryError, served};

impl<C: Catalog, B: BlobStore> Registry<C, B> {
    /// Отдаёт файл версии. Yanked-версия скачивается (lock-файлы должны собираться),
    /// скрыты только не-`Released`.
    pub async fn fetch(
        &self,
        // Чтение пока не авторизуется; в v0.4 Cedar решает и для чтений.
        _caller: &Caller,
        coordinate: &Coordinate,
        path: &str,
    ) -> Result<Download, RegistryError> {
        let version = self.find(coordinate).await?;
        served(&version)?;
        let asset = version
            .assets
            .into_iter()
            .find(|asset| asset.path == path)
            .ok_or(RegistryError::AssetNotFound)?;
        let body = self.blobs.open(&asset.sha256).await?;
        Ok(Download { asset, body })
    }

    /// Версии пакета, видимые клиентам, в порядке публикации.
    /// Неизвестный пакет — пустой список, не ошибка.
    pub async fn versions(
        &self,
        // Чтение пока не авторизуется; в v0.4 Cedar решает и для чтений.
        _caller: &Caller,
        format: Format,
        repository: &str,
        package: &PackageName,
    ) -> Result<Vec<PackageVersion>, RegistryError> {
        let repository = self.find_repository(format, repository).await?;
        let all = self.catalog.versions(repository.id, package).await?;
        Ok(all
            .into_iter()
            .filter(|version| served(version).is_ok())
            .collect())
    }

    /// Пакет целиком: видимые версии и теги. Теги отдаются как есть, включая указывающие на
    /// невидимые версии: какие из них показывать, решает формат.
    /// Неизвестный пакет — пустые версии и теги, не ошибка.
    pub async fn package(
        &self,
        // Чтение пока не авторизуется; в v0.4 Cedar решает и для чтений.
        _caller: &Caller,
        format: Format,
        repository: &str,
        package: &PackageName,
    ) -> Result<PackageView, RegistryError> {
        let repository = self.find_repository(format, repository).await?;
        let all = self.catalog.versions(repository.id, package).await?;
        let tags = self.catalog.tags(repository.id, package).await?;
        Ok(PackageView {
            versions: all
                .into_iter()
                .filter(|version| served(version).is_ok())
                .collect(),
            tags,
        })
    }
}
