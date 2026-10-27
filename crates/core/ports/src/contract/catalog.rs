use std::time::{Duration, SystemTime};

use crane_domain::registry::{
    Asset, Format, FormatMetadata, PackageName, PackageVersion, Repository, RepositoryId,
    RepositoryKind, Sha256, Version, VersionId, VersionState,
};
use uuid::Uuid;

use crate::registry::{Catalog, CatalogError, NewVersion};

/// Проверяет реализацию `Catalog`; `new` создаёт чистый каталог для каждого сценария.
pub async fn catalog_contract<C: Catalog>(new: impl Fn() -> C) {
    ensure_repository_returns_new_repository_as_given(&new()).await;
    ensure_repository_with_same_format_and_name_returns_stored_and_ignores_new_id(&new()).await;
    find_repository_by_format_and_name_returns_stored(&new()).await;
    find_repository_with_other_name_is_none(&new()).await;
    find_repository_with_other_format_is_none(&new()).await;
    publish_returns_record_with_given_fields_and_not_yanked(&new()).await;
    find_version_returns_published_record_unchanged(&new()).await;
    publish_of_same_canonical_version_is_version_exists(&new()).await;
    publish_under_other_spelling_of_name_is_version_exists(&new()).await;
    publish_over_deleted_version_is_version_exists(&new()).await;
    publish_under_other_spelling_keeps_first_written_name(&new()).await;
    publish_in_other_repository_with_same_name_and_version_succeeds(&new()).await;
    publish_to_unknown_repository_is_not_found(&new()).await;
    find_version_returns_original_spellings(&new()).await;
    find_version_in_other_repository_is_none(&new()).await;
    find_version_of_other_version_is_none(&new()).await;
    versions_find_package_by_normalized_name(&new()).await;
    versions_are_listed_in_publication_order(&new()).await;
    versions_return_original_spellings(&new()).await;
    versions_include_deleted_versions(&new()).await;
    versions_of_unknown_package_is_empty(&new()).await;
    transition_with_matching_from_changes_state(&new()).await;
    transition_with_stale_from_is_state_conflict_and_keeps_state(&new()).await;
    transition_of_unknown_id_is_not_found(&new()).await;
    set_yanked_true_sets_flag(&new()).await;
    set_yanked_true_twice_keeps_flag(&new()).await;
    set_yanked_false_clears_flag(&new()).await;
    set_yanked_of_unknown_id_is_not_found(&new()).await;
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

fn new_version(seq: u64, repo: RepositoryId, package: &str, raw: &str) -> NewVersion {
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
    }
}

async fn ensure_repo<C: Catalog>(catalog: &C, id: u128, repo_name: &str) -> RepositoryId {
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

async fn publish<C: Catalog>(catalog: &C, new: NewVersion) {
    catalog
        .publish_version(new)
        .await
        .expect("публикация в подготовке сценария должна удаваться");
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
    let id = released.id;
    publish(catalog, released).await;
    catalog
        .transition(id, VersionState::Released, VersionState::Deleted)
        .await
        .expect("transition в Deleted в подготовке сценария");
}

async fn ensure_repository_returns_new_repository_as_given<C: Catalog>(catalog: &C) {
    // Arrange
    let given = repository(1, "main");

    // Act
    let result = catalog.ensure_repository(given.clone()).await;

    // Assert
    assert_eq!(
        result.expect("ensure_repository"),
        given,
        "ensure_repository: новый репозиторий возвращается как передан"
    );
}

async fn ensure_repository_with_same_format_and_name_returns_stored_and_ignores_new_id<
    C: Catalog,
>(
    catalog: &C,
) {
    // Arrange
    let first = repository(1, "main");
    catalog
        .ensure_repository(first.clone())
        .await
        .expect("первый ensure_repository");

    // Act
    let result = catalog.ensure_repository(repository(2, "main")).await;

    // Assert
    assert_eq!(
        result.expect("повторный ensure_repository"),
        first,
        "ensure_repository идемпотентен: при том же (format, name) возвращается сохранённый, переданный id игнорируется"
    );
}

async fn find_repository_by_format_and_name_returns_stored<C: Catalog>(catalog: &C) {
    // Arrange
    let stored = repository(1, "main");
    catalog
        .ensure_repository(stored.clone())
        .await
        .expect("ensure_repository");

    // Act
    let result = catalog.find_repository(Format::Cargo, "main").await;

    // Assert
    assert_eq!(
        result.expect("find_repository"),
        Some(stored),
        "find_repository находит репозиторий по (format, name)"
    );
}

async fn find_repository_with_other_name_is_none<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_repo(catalog, 1, "main").await;

    // Act
    let result = catalog.find_repository(Format::Cargo, "other").await;

    // Assert
    assert_eq!(
        result.expect("find_repository"),
        None,
        "find_repository: чужое имя даёт None"
    );
}

async fn find_repository_with_other_format_is_none<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_repo(catalog, 1, "main").await;

    // Act
    let result = catalog.find_repository(Format::Npm, "main").await;

    // Assert
    assert_eq!(
        result.expect("find_repository"),
        None,
        "find_repository: чужой формат даёт None"
    );
}

async fn publish_returns_record_with_given_fields_and_not_yanked<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");

    // Act
    let result = catalog.publish_version(new.clone()).await;

    // Assert
    let published = result.expect("publish_version");
    assert_eq!(
        published,
        expected_record(&new),
        "publish_version возвращает запись с переданными полями и yanked = false"
    );
    assert_eq!(
        published.package.as_written(),
        "serde",
        "publish_version сохраняет написание имени"
    );
}

async fn find_version_returns_published_record_unchanged<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");
    publish(catalog, new.clone()).await;

    // Act
    let result = catalog.find_version(repo, &new.package, &new.version).await;

    // Assert
    assert_eq!(
        result.expect("find_version"),
        Some(expected_record(&new)),
        "find_version возвращает опубликованную запись без искажений"
    );
}

async fn publish_of_same_canonical_version_is_version_exists<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "serde", "1.0.0+a")).await;

    // Act
    let result = catalog
        .publish_version(new_version(11, repo, "serde", "1.0.0+b"))
        .await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::VersionExists)),
        "уникальность версии считается по canonical(): 1.0.0+a и 1.0.0+b — одна версия, ждали VersionExists, получили {result:?}"
    );
}

async fn publish_under_other_spelling_of_name_is_version_exists<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "Foo-Bar", "1.0.0")).await;

    // Act
    let result = catalog
        .publish_version(new_version(11, repo, "foo_bar", "1.0.0"))
        .await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::VersionExists)),
        "уникальность считается по normalized() имени: Foo-Bar и foo_bar — один пакет, иначе две сущности на одной координате; ждали VersionExists, получили {result:?}"
    );
}

async fn publish_over_deleted_version_is_version_exists<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish_deleted(catalog, repo).await;

    // Act
    let result = catalog
        .publish_version(new_version(11, repo, "serde", "1.0.0"))
        .await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::VersionExists)),
        "tombstone: номер удалённой версии занят навсегда, ждали VersionExists, получили {result:?}"
    );
}

async fn publish_under_other_spelling_keeps_first_written_name<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "Foo-Bar", "1.0.0")).await;

    // Act
    let result = catalog
        .publish_version(new_version(11, repo, "foo_bar", "1.0.1"))
        .await;

    // Assert
    assert_eq!(
        result.expect("вторая публикация").package.as_written(),
        "Foo-Bar",
        "пакет хранит первое написание имени, даже если версию опубликовали под другим"
    );
}

async fn publish_in_other_repository_with_same_name_and_version_succeeds<C: Catalog>(catalog: &C) {
    // Arrange
    let first = ensure_repo(catalog, 1, "main").await;
    let second = ensure_repo(catalog, 2, "mirror").await;
    publish(catalog, new_version(10, first, "serde", "1.0.0")).await;

    // Act
    let result = catalog
        .publish_version(new_version(11, second, "serde", "1.0.0"))
        .await;

    // Assert
    assert!(
        result.is_ok(),
        "изоляция репозиториев: та же пара имя+версия публикуется в другом репозитории, получили {result:?}"
    );
}

async fn publish_to_unknown_repository_is_not_found<C: Catalog>(catalog: &C) {
    // Arrange
    let unknown = RepositoryId::new(Uuid::from_u128(404));

    // Act
    let result = catalog
        .publish_version(new_version(10, unknown, "serde", "1.0.0"))
        .await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::NotFound)),
        "публикация в неизвестный репозиторий даёт NotFound, получили {result:?}"
    );
}

async fn find_version_returns_original_spellings<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(
        catalog,
        new_version(10, repo, "Serde-Json", "1.0.0+build.5"),
    )
    .await;

    // Act
    let result = catalog
        .find_version(repo, &name("serde_json"), &version("1.0.0"))
        .await;

    // Assert
    let found = result
        .expect("find_version")
        .expect("версия должна быть найдена");
    assert_eq!(
        spellings(&found),
        ("Serde-Json", "1.0.0+build.5"),
        "find_version отдаёт исходные написания имени и версии, а не нормализованные"
    );
}

async fn find_version_in_other_repository_is_none<C: Catalog>(catalog: &C) {
    // Arrange
    let repo_a = ensure_repo(catalog, 1, "a").await;
    let repo_b = ensure_repo(catalog, 2, "b").await;
    publish(catalog, new_version(10, repo_a, "serde", "1.0.0")).await;

    // Act
    let result = catalog
        .find_version(repo_b, &name("serde"), &version("1.0.0"))
        .await;

    // Assert
    assert_eq!(
        result.expect("find_version"),
        None,
        "find_version требует совпадения репозитория: чужой репозиторий даёт None"
    );
}

async fn find_version_of_other_version_is_none<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "serde", "1.0.0")).await;

    // Act
    let result = catalog
        .find_version(repo, &name("serde"), &version("2.0.0"))
        .await;

    // Assert
    assert_eq!(
        result.expect("find_version"),
        None,
        "find_version требует совпадения версии: другая версия даёт None"
    );
}

async fn versions_find_package_by_normalized_name<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "Foo-Bar", "1.0.0")).await;

    // Act
    let result = catalog.versions(repo, &name("foo_bar")).await;

    // Assert
    let written: Vec<&str> = result
        .as_ref()
        .expect("versions")
        .iter()
        .map(|v| v.version.as_written())
        .collect();
    assert_eq!(
        written,
        vec!["1.0.0"],
        "versions находит пакет по нормализованному имени, а не по написанию"
    );
}

async fn versions_are_listed_in_publication_order<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "serde", "1.0.0")).await;
    publish(catalog, new_version(11, repo, "serde", "1.0.1")).await;

    // Act
    let result = catalog.versions(repo, &name("serde")).await;

    // Assert
    let written: Vec<&str> = result
        .as_ref()
        .expect("versions")
        .iter()
        .map(|v| v.version.as_written())
        .collect();
    assert_eq!(
        written,
        vec!["1.0.0", "1.0.1"],
        "versions отдаёт версии в порядке публикации"
    );
}

async fn versions_return_original_spellings<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(
        catalog,
        new_version(10, repo, "Serde-Json", "1.0.0+build.5"),
    )
    .await;

    // Act
    let result = catalog.versions(repo, &name("serde_json")).await;

    // Assert
    let listed = result.expect("versions");
    assert_eq!(
        listed.iter().map(spellings).collect::<Vec<_>>(),
        vec![("Serde-Json", "1.0.0+build.5")],
        "versions отдаёт исходные написания имени и версии, а не нормализованные"
    );
}

async fn versions_include_deleted_versions<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish_deleted(catalog, repo).await;

    // Act
    let result = catalog.versions(repo, &name("serde")).await;

    // Assert
    let states: Vec<VersionState> = result.expect("versions").iter().map(|v| v.state).collect();
    assert_eq!(
        states,
        vec![VersionState::Deleted],
        "versions возвращает и Deleted-версии"
    );
}

async fn versions_of_unknown_package_is_empty<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;

    // Act
    let result = catalog.versions(repo, &name("absent")).await;

    // Assert
    let all = result.expect("versions");
    assert!(
        all.is_empty(),
        "versions неизвестного пакета даёт пустой вектор, получили {all:?}"
    );
}

async fn transition_with_matching_from_changes_state<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");
    let id = new.id;
    publish(catalog, new).await;

    // Act
    let result = catalog
        .transition(id, VersionState::Quarantined, VersionState::Released)
        .await;

    // Assert
    result.expect("transition с верным from");
    assert_eq!(
        find(catalog, repo, "serde", "1.0.0").await.state,
        VersionState::Released,
        "transition с верным from меняет состояние"
    );
}

async fn transition_with_stale_from_is_state_conflict_and_keeps_state<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");
    let id = new.id;
    publish(catalog, new).await;
    catalog
        .transition(id, VersionState::Quarantined, VersionState::Released)
        .await
        .expect("transition в Released в подготовке сценария");

    // Act
    let result = catalog
        .transition(id, VersionState::Quarantined, VersionState::Blocked)
        .await;

    // Assert
    assert!(
        matches!(
            result,
            Err(CatalogError::StateConflict {
                actual: VersionState::Released
            })
        ),
        "CAS: устаревший from даёт StateConflict с фактическим состоянием, получили {result:?}"
    );
    assert_eq!(
        find(catalog, repo, "serde", "1.0.0").await.state,
        VersionState::Released,
        "CAS: при конфликте состояние не меняется"
    );
}

async fn transition_of_unknown_id_is_not_found<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_repo(catalog, 1, "main").await;

    // Act
    let result = catalog
        .transition(
            unknown_version_id(),
            VersionState::Quarantined,
            VersionState::Released,
        )
        .await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::NotFound)),
        "transition неизвестного id даёт NotFound, получили {result:?}"
    );
}

async fn set_yanked_true_sets_flag<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");
    let id = new.id;
    publish(catalog, new).await;

    // Act
    let result = catalog.set_yanked(id, true).await;

    // Assert
    result.expect("set_yanked(true)");
    assert!(
        find(catalog, repo, "serde", "1.0.0").await.yanked,
        "set_yanked(true) ставит флаг"
    );
}

async fn set_yanked_true_twice_keeps_flag<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");
    let id = new.id;
    publish(catalog, new).await;
    catalog
        .set_yanked(id, true)
        .await
        .expect("первый set_yanked(true)");

    // Act
    let result = catalog.set_yanked(id, true).await;

    // Assert
    result.expect("повторный set_yanked(true)");
    assert!(
        find(catalog, repo, "serde", "1.0.0").await.yanked,
        "set_yanked идемпотентен: повтор не меняет результат"
    );
}

async fn set_yanked_false_clears_flag<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = new_version(10, repo, "serde", "1.0.0");
    let id = new.id;
    publish(catalog, new).await;
    catalog
        .set_yanked(id, true)
        .await
        .expect("set_yanked(true) в подготовке сценария");

    // Act
    let result = catalog.set_yanked(id, false).await;

    // Assert
    result.expect("set_yanked(false)");
    assert!(
        !find(catalog, repo, "serde", "1.0.0").await.yanked,
        "set_yanked(false) снимает флаг"
    );
}

async fn set_yanked_of_unknown_id_is_not_found<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_repo(catalog, 1, "main").await;

    // Act
    let result = catalog.set_yanked(unknown_version_id(), true).await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::NotFound)),
        "set_yanked неизвестного id даёт NotFound, получили {result:?}"
    );
}
