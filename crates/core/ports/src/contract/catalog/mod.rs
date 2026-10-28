mod content;
mod flags;

use std::time::{Duration, SystemTime};

use crane_domain::registry::{
    Asset, Format, FormatMetadata, PackageName, PackageVersion, Repository, RepositoryId,
    RepositoryKind, Sha256, Version, VersionId, VersionState,
};
use uuid::Uuid;

use crate::registry::{Catalog, NewVersion};

/// Проверяет реализацию `Catalog`; `new` создаёт чистый каталог для каждого сценария.
pub async fn catalog_contract<C: Catalog>(new: impl Fn() -> C) {
    content::scenarios(&new).await;
    flags::scenarios(&new).await;
}

pub(super) fn name(raw: &str) -> PackageName {
    PackageName::parse(Format::Cargo, raw).expect("тестовое имя пакета валидно")
}

fn version(raw: &str) -> Version {
    Version::parse(Format::Cargo, raw).expect("тестовая версия валидна")
}

pub(super) fn repository(id: u128, name: &str) -> Repository {
    Repository {
        id: RepositoryId::new(Uuid::from_u128(id)),
        format: Format::Cargo,
        kind: RepositoryKind::Hosted,
        name: name.to_owned(),
    }
}

pub(super) fn new_version(seq: u64, repo: RepositoryId, package: &str, raw: &str) -> NewVersion {
    NewVersion {
        id: VersionId::new(Uuid::from_u128(seq.into())),
        repository: repo,
        package: name(package),
        version: version(raw),
        state: VersionState::Quarantined,
        // Разные секунды у разных seq, чтобы порядок публикации не зависел от равенства времён.
        published_at: SystemTime::UNIX_EPOCH + Duration::from_secs(seq),
        assets: vec![Asset {
            path: format!("{package}-{raw}.crate"),
            sha256: Sha256::from([7; 32]),
            size: 3,
        }],
        metadata: FormatMetadata::new(r#"{"deps":[]}"#.to_owned()),
        tags: Vec::new(),
    }
}

pub(super) async fn ensure_repo<C: Catalog>(
    catalog: &C,
    id: u128,
    repo_name: &str,
) -> RepositoryId {
    catalog
        .ensure_repository(repository(id, repo_name))
        .await
        .expect("ensure_repository новому репозиторию должен удаваться")
        .id
}

async fn find<C: Catalog>(
    catalog: &C,
    repo: RepositoryId,
    package: &str,
    raw: &str,
) -> PackageVersion {
    catalog
        .find_version(repo, &name(package), &version(raw))
        .await
        .expect("find_version не должен падать")
        .expect("версия должна быть найдена")
}

pub(super) async fn publish<C: Catalog>(catalog: &C, new: NewVersion) -> PackageVersion {
    catalog
        .publish_version(new)
        .await
        .expect("публикация в подготовке сценария должна удаваться")
}

/// Запись, которую каталог обязан вернуть после публикации `new`.
fn expected_record(new: &NewVersion) -> PackageVersion {
    PackageVersion {
        id: new.id,
        repository: new.repository,
        package: new.package.clone(),
        version: new.version.clone(),
        state: new.state,
        yanked: false,
        deprecated: None,
        published_at: new.published_at,
        assets: new.assets.clone(),
        metadata: new.metadata.clone(),
    }
}

fn unknown_version_id() -> VersionId {
    VersionId::new(Uuid::from_u128(999))
}

/// Пара (написание имени, написание версии) для сравнения: `Eq` у них семантический.
fn spellings(record: &PackageVersion) -> (&str, &str) {
    (record.package.as_written(), record.version.as_written())
}

/// Публикует `serde 1.0.0` и переводит в `Deleted` (tombstone).
async fn publish_deleted<C: Catalog>(catalog: &C, repo: RepositoryId) {
    let mut released = new_version(10, repo, "serde", "1.0.0");
    released.state = VersionState::Released;
    let id = publish(catalog, released).await.id;
    catalog
        .transition(id, VersionState::Released, VersionState::Deleted)
        .await
        .expect("transition в Deleted в подготовке сценария");
}
