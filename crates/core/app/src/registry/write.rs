use std::time::SystemTime;

use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{
    Asset, Coordinate, DeletePolicy, Format, PackageName, PackageVersion, RepositoryKind,
    VersionId, VersionState,
};
use crane_ports::registry::{BlobStore, Catalog, CatalogError, NewVersion};

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
            tags,
        } = request;
        let repository = self
            .find_repository(coordinate.format, &coordinate.repository)
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
                tags,
            })
            .await?;
        Ok(published)
    }

    /// Складывает тело во временную зону, сверяет заявленные клиентом суммы и только
    /// после совпадения коммитит blob в CAS: неверное содержимое в CAS не попадает.
    async fn ingest(&self, upload: Upload) -> Result<Asset, RegistryError> {
        let staged = self.blobs.stage(upload.body).await?;
        // Проверяется каждая заявленная сумма: клиент мог прислать и sha512, и sha1.
        let digests = staged.digests();
        if let Some(declared) = upload.declared.iter().find(|c| !digests.satisfies(c)) {
            return Err(RegistryError::DigestMismatch {
                declared: Box::new(*declared),
                actual: Box::new(digests.of_kind(declared)),
            });
        }
        let digests = self.blobs.commit(staged).await?;
        Ok(Asset {
            path: upload.path,
            sha256: digests.sha256,
            size: digests.size,
        })
    }

    /// Удаляет версию (tombstone) в окне политики; дальше доступна только блокировка.
    /// Идемпотентно: удалённая версия — успех (событие пишется и для повтора).
    pub async fn delete(
        &self,
        caller: &Caller,
        coordinate: &Coordinate,
        policy: &DeletePolicy,
    ) -> Result<(), RegistryError> {
        self.delete_at(caller, coordinate, policy, self.clock.now())
            .await
    }

    /// Удаляет все версии пакета. Окно проверяется для всех до первой мутации: если хоть одна
    /// неудалённая версия вне окна, не меняется ничего и пишется одно событие отказа.
    /// Повтор после сбоя безопасен: уже удалённые версии пропускаются.
    /// Неизвестный пакет и не-hosted репозиторий событий не пишут: координаты версии нет.
    pub async fn delete_package(
        &self,
        caller: &Caller,
        format: Format,
        repository: &str,
        package: &PackageName,
        policy: &DeletePolicy,
    ) -> Result<(), RegistryError> {
        let repo = self.find_repository(format, repository).await?;
        if repo.kind != RepositoryKind::Hosted {
            return Err(RegistryError::NotHosted);
        }
        let versions = self.catalog.versions(repo.id, package).await?;
        if versions.is_empty() {
            return Err(RegistryError::VersionNotFound);
        }
        // Одно значение `now` на весь вызов: решение «в окне» не должно меняться по ходу цепочки.
        let now = self.clock.now();
        let coordinate_of = |v: &PackageVersion| Coordinate {
            format,
            repository: repository.to_owned(),
            package: v.package.clone(),
            version: v.version.clone(),
        };
        let pending: Vec<&PackageVersion> = versions
            .iter()
            .filter(|v| v.state != VersionState::Deleted)
            .collect();
        // Те же отказы, что у `delete`, но для всех версий до первой мутации: цепочка не должна
        // оставлять пакет наполовину удалённым.
        let refusal = pending
            .iter()
            .find_map(|v| refusal(v, policy, now).map(|error| (coordinate_of(v), error)));
        if let Some((coordinate, error)) = refusal {
            let refused: Result<(), RegistryError> = Err(error);
            self.audited(caller, AuditAction::Delete(coordinate), &refused);
            return refused;
        }
        for version in pending {
            self.delete_at(caller, &coordinate_of(version), policy, now)
                .await?;
        }
        Ok(())
    }

    async fn delete_at(
        &self,
        caller: &Caller,
        coordinate: &Coordinate,
        policy: &DeletePolicy,
        now: SystemTime,
    ) -> Result<(), RegistryError> {
        let result = self.try_delete(coordinate, policy, now).await;
        self.audited(caller, AuditAction::Delete(coordinate.clone()), &result);
        result
    }

    async fn try_delete(
        &self,
        coordinate: &Coordinate,
        policy: &DeletePolicy,
        now: SystemTime,
    ) -> Result<(), RegistryError> {
        let version = self.find_hosted(coordinate).await?;
        if version.state == VersionState::Deleted {
            return Ok(());
        }
        if let Some(error) = refusal(&version, policy, now) {
            return Err(error);
        }
        match self
            .catalog
            .transition(version.id, version.state, VersionState::Deleted)
            .await
        {
            // Параллельное удаление той же версии даёт тот же итог.
            Ok(())
            | Err(CatalogError::StateConflict {
                actual: VersionState::Deleted,
            }) => Ok(()),
            Err(CatalogError::StateConflict { actual }) => {
                Err(RegistryError::Unavailable { state: actual })
            }
            Err(error) => Err(error.into()),
        }
    }
}

/// Почему версию нельзя удалить сейчас: окно закрыто или состояние не ведёт в `Deleted`.
fn refusal(
    version: &PackageVersion,
    policy: &DeletePolicy,
    now: SystemTime,
) -> Option<RegistryError> {
    if !policy.permits(version.published_at, now) {
        return Some(RegistryError::DeleteWindowClosed);
    }
    version
        .state
        .ensure(VersionState::Deleted)
        .err()
        .map(|_| RegistryError::Unavailable {
            state: version.state,
        })
}
