//! Общие шаги записи: публикация и прикрепление файла, удаление живут в соседних модулях.

mod attach;
mod delete;
mod publish;

use crane_domain::access::Caller;
use crane_domain::audit::AuditAction;
use crane_domain::registry::{
    Asset, Format, FormatMetadata, Namespace, NamespacePattern, PackageName, PackageVersion,
    Repository, VersionId, VersionState, hosted_name_allowed,
};
use crane_ports::proxy::Presence;
use crane_ports::registry::{BlobStore, Catalog, CatalogError, NewVersion};

use super::{Coordinate, Registry, RegistryError, Upload};

pub use attach::Attach;

/// Отказ по правилу формата: код стабилен, причина безопасна для журнала аудита.
#[derive(Debug)]
pub struct Refusal {
    pub code: &'static str,
    pub reason: String,
}

impl<C: Catalog, B: BlobStore> Registry<C, B> {
    /// Создаёт версию с одним файлом. Ошибка порта отдаётся как есть: `attach` отличает
    /// `VersionExists` (гонка первых загрузок), `publish` просто преобразует её.
    async fn create_version(
        &self,
        repository: &Repository,
        coordinate: &Coordinate,
        asset: Asset,
        metadata: FormatMetadata,
        tags: Vec<String>,
    ) -> Result<PackageVersion, CatalogError> {
        self.catalog
            .publish_version(NewVersion {
                id: VersionId::new(self.ids.generate()),
                repository: repository.id,
                package: coordinate.package.clone(),
                version: coordinate.version.clone(),
                // Hosted без gates в v0.1.0 публикует сразу в Released; quarantine заменит эту строку.
                state: VersionState::Released,
                published_at: self.clock.now(),
                assets: vec![asset],
                metadata,
                tags,
            })
            .await
    }

    /// Событие use case и, если запись захватила новое имя, отдельный `Reserve`.
    /// Отступление от «одно событие на use case»: захват имени - отдельный факт безопасности
    /// (с него имя становится внутренним), и аудитору он нужен самостоятельной записью,
    /// а не догадкой по первой загрузке. Отказ захвата не оставляет.
    fn audited_claim(
        &self,
        caller: &Caller,
        action: AuditAction,
        outcome: Result<(PackageVersion, Option<Namespace>), RegistryError>,
    ) -> Result<PackageVersion, RegistryError> {
        let (result, claim) = match outcome {
            Ok((version, claim)) => (Ok(version), claim),
            Err(error) => (Err(error), None),
        };
        self.audited(caller, action, &result);
        if let Some(claim) = claim {
            self.audited(caller, AuditAction::Reserve(claim), &result);
        }
        result
    }

    /// Политика формата проверяется до любых обращений к хранилищам: отказ ничего не пишет.
    fn accepted(
        metadata: Result<FormatMetadata, Refusal>,
    ) -> Result<FormatMetadata, RegistryError> {
        metadata.map_err(|refusal| RegistryError::Rejected {
            code: refusal.code,
            reason: refusal.reason,
        })
    }

    /// Защита от dependency confusion при публикации: имя должно быть допустимо для формата,
    /// не принадлежать чужому namespace, а новое имя - не быть известно публичному upstream.
    /// Возвращает namespace точного имени, если публикация захватывает новое имя.
    ///
    /// Захват не пишет отдельной строки: первая версия создаёт строку пакета атомарно
    /// в `publish_version`, и эта строка (их не удаляют) и есть резервирование.
    async fn admit_publication(
        &self,
        repository: &Repository,
        format: Format,
        package: &PackageName,
    ) -> Result<Option<Namespace>, RegistryError> {
        let eco = (self.ecosystems)(format);
        if !hosted_name_allowed(eco, package) {
            return Err(RegistryError::UnscopedName);
        }
        let candidates = NamespacePattern::covering(eco, package);
        let claim = match self.catalog.namespace_for(format, &candidates).await? {
            Some(namespace) if namespace.workspace != repository.workspace() => {
                return Err(RegistryError::NamespaceOwned);
            }
            // Имя под собственным namespace уже зарезервировано: upstream не спрашиваем.
            Some(_) => false,
            None if self.catalog.hosted_exists(format, package).await? => false,
            None => match self.upstream.presence(format, package).await {
                Presence::Absent => true,
                Presence::Public => return Err(RegistryError::PublicName),
                Presence::Unknown => return Err(RegistryError::UpstreamUnavailable),
            },
        };
        Ok(claim.then(|| Namespace {
            format,
            pattern: NamespacePattern::exact(package),
            workspace: repository.workspace(),
        }))
    }

    /// Резервирует namespace: идемпотентно по `(format, pattern)`; чужой владелец даёт
    /// `NamespaceConflict`. Аудируется всегда, успех и отказ.
    ///
    /// В v0.2.0 вызывает только конфигурация сервера (власть платформы).
    // Крючок v0.4.0: пересечения и вырезание вложенного namespace чужого workspace
    // (только platform-admin со step-up) и решение Cedar - здесь, до `ensure_namespace`.
    pub async fn reserve(
        &self,
        caller: &Caller,
        namespace: Namespace,
    ) -> Result<Namespace, RegistryError> {
        let action = AuditAction::Reserve(namespace.clone());
        let requested = namespace.workspace;
        let result = match self.catalog.ensure_namespace(namespace).await {
            Ok(stored) if stored.workspace == requested => Ok(stored),
            Ok(_) => Err(RegistryError::NamespaceConflict),
            Err(error) => Err(error.into()),
        };
        self.audited(caller, action, &result);
        result
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
            content: digests,
        })
    }
}
