use std::time::SystemTime;

use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{
    Coordinate, DeletePolicy, Format, PackageName, PackageVersion, RepositoryKind, VersionState,
};
use crane_ports::registry::{BlobStore, Catalog, CatalogError};

use super::{Registry, RegistryError};

impl<C: Catalog, B: BlobStore> Registry<C, B> {
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
