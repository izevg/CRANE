use std::time::{Duration, SystemTime};

use crane_domain::{
    Asset, Format, FormatMetadata, PackageName, PackageVersion, Repository, RepositoryId,
    RepositoryKind, Sha256, Version, VersionId, VersionState,
};
use uuid::Uuid;

use crate::{Catalog, CatalogError, NewVersion};

/// Проверяет реализацию `Catalog`; `new` создаёт чистый каталог для каждого сценария.
pub async fn catalog_contract<C: Catalog>(new: impl Fn() -> C) {
    ensure_repository_is_idempotent(&new()).await;
    published_version_is_found_with_same_fields(&new()).await;
    version_is_unique_by_canonical_form(&new()).await;
    deleted_version_stays_reserved(&new()).await;
    package_keeps_first_written_name(&new()).await;
    names_differing_only_in_spelling_collide(&new()).await;
    repositories_are_isolated(&new()).await;
    transition_is_compare_and_set(&new()).await;
    set_yanked_is_idempotent(&new()).await;
    publish_to_unknown_repository_is_not_found(&new()).await;
    versions_of_unknown_package_is_empty(&new()).await;
}

fn name(raw: &str) -> PackageName {
    PackageName::parse(Format::Cargo, raw).expect("тестовое имя пакета валидно")
}

fn version(raw: &str) -> Version {
    Version::parse(Format::Cargo, raw).expect("тестовая версия валидна")
}

fn repository(id: u128, name: &str) -> Repository {
    Repository {
        id: RepositoryId::new(Uuid::from_u128(id)),
        format: Format::Cargo,
        kind: RepositoryKind::Hosted,
        name: name.to_owned(),
    }
}

fn new_version(id: u128, repo: RepositoryId, package: &str, raw: &str) -> NewVersion {
    NewVersion {
        id: VersionId::new(Uuid::from_u128(id)),
        repository: repo,
        package: name(package),
        version: version(raw),
        state: VersionState::Quarantined,
        // Разные секунды у разных id, чтобы порядок публикации не зависел от равенства времён.
        published_at: SystemTime::UNIX_EPOCH + Duration::from_secs(u64::try_from(id).unwrap_or(0)),
        assets: vec![Asset {
            path: format!("{package}-{raw}.crate"),
            sha256: Sha256::from([7; 32]),
            size: 3,
        }],
        metadata: FormatMetadata::new(r#"{"deps":[]}"#.to_owned()),
    }
}

async fn ensure_repo<C: Catalog>(catalog: &C, id: u128, repo_name: &str) -> RepositoryId {
    catalog
        .ensure_repository(repository(id, repo_name))
        .await
        .expect("ensure_repository новому репозиторию должен удаваться")
        .id
}

async fn state_of<C: Catalog>(
    catalog: &C,
    repo: RepositoryId,
    package: &str,
    raw: &str,
) -> VersionState {
    catalog
        .find_version(repo, &name(package), &version(raw))
        .await
        .expect("find_version не должен падать")
        .expect("версия должна быть найдена")
        .state
}

async fn ensure_repository_is_idempotent<C: Catalog>(catalog: &C) {
    let first = repository(1, "main");
    let stored = catalog
        .ensure_repository(first.clone())
        .await
        .expect("ensure_repository");
    assert_eq!(
        stored, first,
        "ensure_repository: новый репозиторий возвращается как передан"
    );

    let again = catalog
        .ensure_repository(repository(2, "main"))
        .await
        .expect("повторный ensure_repository");
    assert_eq!(
        again, first,
        "ensure_repository идемпотентен: при том же (format, name) возвращается сохранённый, переданный id игнорируется"
    );

    let found = catalog
        .find_repository(Format::Cargo, "main")
        .await
        .expect("find_repository");
    assert_eq!(
        found,
        Some(first),
        "find_repository находит репозиторий по (format, name)"
    );

    let other_name = catalog
        .find_repository(Format::Cargo, "other")
        .await
        .expect("find_repository");
    assert_eq!(other_name, None, "find_repository: чужое имя даёт None");

    let other_format = catalog
        .find_repository(Format::Npm, "main")
        .await
        .expect("find_repository");
    assert_eq!(
        other_format, None,
        "find_repository: чужой формат даёт None"
    );
}

async fn published_version_is_found_with_same_fields<C: Catalog>(catalog: &C) {
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");

    let published = catalog
        .publish_version(new.clone())
        .await
        .expect("publish_version");

    let expected = PackageVersion {
        id: new.id,
        repository: new.repository,
        package: new.package.clone(),
        version: new.version.clone(),
        state: new.state,
        yanked: false,
        published_at: new.published_at,
        assets: new.assets.clone(),
        metadata: new.metadata.clone(),
    };
    assert_eq!(
        published, expected,
        "publish_version возвращает запись с переданными полями и yanked = false"
    );
    assert_eq!(
        published.package.as_written(),
        "serde",
        "publish_version сохраняет написание имени"
    );
    let found = catalog
        .find_version(repo, &new.package, &new.version)
        .await
        .expect("find_version");
    assert_eq!(
        found,
        Some(expected),
        "find_version возвращает опубликованную запись без искажений"
    );
}

async fn version_is_unique_by_canonical_form<C: Catalog>(catalog: &C) {
    let repo = ensure_repo(catalog, 1, "main").await;
    catalog
        .publish_version(new_version(10, repo, "serde", "1.0.0+a"))
        .await
        .expect("первая публикация");

    let result = catalog
        .publish_version(new_version(11, repo, "serde", "1.0.0+b"))
        .await;

    assert!(
        matches!(result, Err(CatalogError::VersionExists)),
        "уникальность версии считается по canonical(): 1.0.0+a и 1.0.0+b — одна версия, ждали VersionExists, получили {result:?}"
    );
}

async fn deleted_version_stays_reserved<C: Catalog>(catalog: &C) {
    let repo = ensure_repo(catalog, 1, "main").await;
    let mut released = new_version(10, repo, "serde", "1.0.0");
    released.state = VersionState::Released;
    let id = released.id;
    catalog.publish_version(released).await.expect("публикация");
    catalog
        .transition(id, VersionState::Released, VersionState::Deleted)
        .await
        .expect("transition в Deleted");

    let result = catalog
        .publish_version(new_version(11, repo, "serde", "1.0.0"))
        .await;

    assert!(
        matches!(result, Err(CatalogError::VersionExists)),
        "tombstone: номер удалённой версии занят навсегда, ждали VersionExists, получили {result:?}"
    );
    let all = catalog
        .versions(repo, &name("serde"))
        .await
        .expect("versions");
    assert_eq!(
        all.iter().map(|v| v.state).collect::<Vec<_>>(),
        vec![VersionState::Deleted],
        "versions возвращает и Deleted-версии"
    );
}

async fn package_keeps_first_written_name<C: Catalog>(catalog: &C) {
    let repo = ensure_repo(catalog, 1, "main").await;
    catalog
        .publish_version(new_version(10, repo, "Foo-Bar", "1.0.0"))
        .await
        .expect("первая публикация");

    let second = catalog
        .publish_version(new_version(11, repo, "foo_bar", "1.0.1"))
        .await
        .expect("вторая публикация");

    assert_eq!(
        second.package.as_written(),
        "Foo-Bar",
        "пакет хранит первое написание имени, даже если версию опубликовали под другим"
    );
    let all = catalog
        .versions(repo, &name("foo_bar"))
        .await
        .expect("versions");
    let written: Vec<&str> = all.iter().map(|v| v.version.as_written()).collect();
    assert_eq!(
        written,
        vec!["1.0.0", "1.0.1"],
        "versions находит пакет по нормализованному имени и отдаёт версии в порядке публикации"
    );
}

async fn names_differing_only_in_spelling_collide<C: Catalog>(catalog: &C) {
    let repo = ensure_repo(catalog, 1, "main").await;
    catalog
        .publish_version(new_version(10, repo, "Foo-Bar", "1.0.0"))
        .await
        .expect("первая публикация");

    let result = catalog
        .publish_version(new_version(11, repo, "foo_bar", "1.0.0"))
        .await;

    assert!(
        matches!(result, Err(CatalogError::VersionExists)),
        "уникальность считается по normalized() имени: Foo-Bar и foo_bar — один пакет, иначе две сущности на одной координате; ждали VersionExists, получили {result:?}"
    );
}

async fn repositories_are_isolated<C: Catalog>(catalog: &C) {
    let first = ensure_repo(catalog, 1, "main").await;
    let second = ensure_repo(catalog, 2, "mirror").await;

    let in_first = catalog
        .publish_version(new_version(10, first, "serde", "1.0.0"))
        .await;
    let in_second = catalog
        .publish_version(new_version(11, second, "serde", "1.0.0"))
        .await;

    assert!(
        in_first.is_ok(),
        "публикация в первом репозитории: {in_first:?}"
    );
    assert!(
        in_second.is_ok(),
        "изоляция репозиториев: та же пара имя+версия публикуется в другом репозитории, получили {in_second:?}"
    );
}

async fn transition_is_compare_and_set<C: Catalog>(catalog: &C) {
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");
    let id = new.id;
    catalog.publish_version(new).await.expect("публикация");

    catalog
        .transition(id, VersionState::Quarantined, VersionState::Released)
        .await
        .expect("transition с верным from");
    assert_eq!(
        state_of(catalog, repo, "serde", "1.0.0").await,
        VersionState::Released,
        "transition с верным from меняет состояние"
    );

    let stale = catalog
        .transition(id, VersionState::Quarantined, VersionState::Blocked)
        .await;
    assert!(
        matches!(
            stale,
            Err(CatalogError::StateConflict {
                actual: VersionState::Released
            })
        ),
        "CAS: устаревший from даёт StateConflict с фактическим состоянием, получили {stale:?}"
    );
    assert_eq!(
        state_of(catalog, repo, "serde", "1.0.0").await,
        VersionState::Released,
        "CAS: при конфликте состояние не меняется"
    );

    let unknown = catalog
        .transition(
            VersionId::new(Uuid::from_u128(999)),
            VersionState::Quarantined,
            VersionState::Released,
        )
        .await;
    assert!(
        matches!(unknown, Err(CatalogError::NotFound)),
        "transition неизвестного id даёт NotFound, получили {unknown:?}"
    );
}

async fn set_yanked_is_idempotent<C: Catalog>(catalog: &C) {
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");
    let id = new.id;
    catalog.publish_version(new).await.expect("публикация");
    let yanked = |expected: bool, rule: &'static str| async move {
        let found = catalog
            .find_version(repo, &name("serde"), &version("1.0.0"))
            .await
            .expect("find_version")
            .expect("версия найдена");
        assert_eq!(found.yanked, expected, "{rule}");
    };

    catalog
        .set_yanked(id, true)
        .await
        .expect("set_yanked(true)");
    yanked(true, "set_yanked(true) ставит флаг").await;
    catalog
        .set_yanked(id, true)
        .await
        .expect("повторный set_yanked(true)");
    yanked(true, "set_yanked идемпотентен: повтор не меняет результат").await;
    catalog
        .set_yanked(id, false)
        .await
        .expect("set_yanked(false)");
    yanked(false, "set_yanked(false) снимает флаг").await;

    let unknown = catalog
        .set_yanked(VersionId::new(Uuid::from_u128(999)), true)
        .await;
    assert!(
        matches!(unknown, Err(CatalogError::NotFound)),
        "set_yanked неизвестного id даёт NotFound, получили {unknown:?}"
    );
}

async fn publish_to_unknown_repository_is_not_found<C: Catalog>(catalog: &C) {
    let unknown = RepositoryId::new(Uuid::from_u128(404));

    let result = catalog
        .publish_version(new_version(10, unknown, "serde", "1.0.0"))
        .await;

    assert!(
        matches!(result, Err(CatalogError::NotFound)),
        "публикация в неизвестный репозиторий даёт NotFound, получили {result:?}"
    );
}

async fn versions_of_unknown_package_is_empty<C: Catalog>(catalog: &C) {
    let repo = ensure_repo(catalog, 1, "main").await;

    let all = catalog
        .versions(repo, &name("absent"))
        .await
        .expect("versions");

    assert!(
        all.is_empty(),
        "versions неизвестного пакета даёт пустой вектор, получили {all:?}"
    );
}
