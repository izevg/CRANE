use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{Format, NamespacePattern, PackageName, PackageVersion};
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

impl<C: Catalog, B: BlobStore> Registry<C, B> {
    /// Можно ли идти за именем в публичный upstream. Внутреннее имя (покрыто namespace
    /// или есть в любом hosted-репозитории формата) оттуда не отдаётся никогда:
    /// иначе чужой пакет с тем же именем подменил бы внутренний.
    ///
    /// Сбой каталога - `Backend`: вызывающий обязан не ходить в upstream (fail-closed).
    /// Отказ аудируется как `ShadowUpstream`, разрешение - нет (чтения не аудируются).
    pub async fn admit_upstream(
        &self,
        caller: &Caller,
        format: Format,
        name: &PackageName,
    ) -> Result<(), RegistryError> {
        if !self.is_internal(format, name).await? {
            return Ok(());
        }
        let result = Err(RegistryError::Shadowed);
        let action = AuditAction::ShadowUpstream {
            format,
            package: name.clone(),
        };
        self.audited(caller, action, &result);
        result
    }

    async fn is_internal(&self, format: Format, name: &PackageName) -> Result<bool, RegistryError> {
        // Короткое замыкание: namespace дешевле, чем поиск по пакетам.
        let eco = (self.ecosystems)(format);
        let candidates = NamespacePattern::covering(eco, name);
        Ok(self
            .catalog
            .namespace_for(format, &candidates)
            .await?
            .is_some()
            || self.catalog.hosted_exists(format, name).await?)
    }
}
