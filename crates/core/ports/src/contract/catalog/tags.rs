use crane_domain::registry::{RepositoryId, VersionId};
use uuid::Uuid;

use super::{ensure_repo, find, name, new_version, publish, unknown_version_id};
use crate::registry::{Catalog, CatalogError, NewVersion};

/// Сценарии группы; порядок вызовов сохраняет порядок общего прогона.
pub(super) async fn scenarios<C: Catalog>(new: &impl Fn() -> C) {
    publish_with_tags_sets_them_sorted_by_tag(&new()).await;
    publish_with_existing_tag_moves_it_to_new_version(&new()).await;
    publish_without_tags_leaves_existing_tags_untouched(&new()).await;
    rejected_publish_does_not_move_tags(&new()).await;
    set_tag_creates_tag_on_version(&new()).await;
    set_tag_moves_existing_tag_and_is_idempotent(&new()).await;
    set_tag_of_unknown_version_is_not_found(&new()).await;
    tags_belong_to_the_package_of_their_version(&new()).await;
    remove_tag_removes_only_that_tag(&new()).await;
    remove_absent_tag_is_false(&new()).await;
    tags_of_unknown_package_is_empty(&new()).await;
    set_deprecated_sets_message_visible_in_find_and_versions(&new()).await;
    set_deprecated_none_clears_message_and_is_idempotent(&new()).await;
    set_deprecated_of_unknown_id_is_not_found(&new()).await;
}

fn tagged(mut new: NewVersion, tags: &[&str]) -> NewVersion {
    new.tags = tags.iter().map(|t| (*t).to_owned()).collect();
    new
}

async fn tags_of<C: Catalog>(
    catalog: &C,
    repo: RepositoryId,
    package: &str,
) -> Vec<(String, VersionId)> {
    catalog
        .tags(repo, &name(package))
        .await
        .expect("tags не должен падать")
}

fn pair(tag: &str, seq: u128) -> (String, VersionId) {
    (tag.to_owned(), VersionId::new(Uuid::from_u128(seq)))
}

async fn publish_with_tags_sets_them_sorted_by_tag<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let new = tagged(
        new_version(10, repo, "serde", "1.0.0"),
        &["next", "latest", "next"],
    );

    // Act
    let result = catalog.publish_version(new).await;

    // Assert
    result.expect("публикация с тегами");
    assert_eq!(
        tags_of(catalog, repo, "serde").await,
        vec![pair("latest", 10), pair("next", 10)],
        "теги публикации стоят на новой версии, повтор имени схлопывается, порядок по имени"
    );
}

async fn publish_with_existing_tag_moves_it_to_new_version<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(
        catalog,
        tagged(
            new_version(10, repo, "serde", "1.0.0"),
            &["latest", "stable"],
        ),
    )
    .await;

    // Act
    let result = catalog
        .publish_version(tagged(new_version(11, repo, "serde", "2.0.0"), &["latest"]))
        .await;

    // Assert
    result.expect("публикация второй версии");
    assert_eq!(
        tags_of(catalog, repo, "serde").await,
        vec![pair("latest", 11), pair("stable", 10)],
        "тег переносится на новую версию, остальные теги остаются"
    );
}

async fn publish_without_tags_leaves_existing_tags_untouched<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(
        catalog,
        tagged(new_version(10, repo, "serde", "1.0.0"), &["latest"]),
    )
    .await;

    // Act
    let result = catalog
        .publish_version(new_version(11, repo, "serde", "2.0.0"))
        .await;

    // Assert
    result.expect("публикация без тегов");
    assert_eq!(
        tags_of(catalog, repo, "serde").await,
        vec![pair("latest", 10)],
        "публикация без тегов теги не трогает"
    );
}

async fn rejected_publish_does_not_move_tags<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(
        catalog,
        tagged(new_version(10, repo, "serde", "1.0.0"), &["latest"]),
    )
    .await;

    // Act
    let result = catalog
        .publish_version(tagged(new_version(11, repo, "serde", "1.0.0"), &["latest"]))
        .await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::VersionExists)),
        "дубликат версии отклоняется, получили {result:?}"
    );
    assert_eq!(
        tags_of(catalog, repo, "serde").await,
        vec![pair("latest", 10)],
        "отклонённая публикация не переносит теги"
    );
}

async fn set_tag_creates_tag_on_version<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let id = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;

    // Act
    let result = catalog.set_tag(id, "beta").await;

    // Assert
    result.expect("set_tag");
    assert_eq!(
        tags_of(catalog, repo, "serde").await,
        vec![pair("beta", 10)],
        "set_tag ставит тег на версию"
    );
}

async fn set_tag_moves_existing_tag_and_is_idempotent<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let first = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;
    let second = publish(catalog, new_version(11, repo, "serde", "2.0.0"))
        .await
        .id;
    catalog
        .set_tag(first, "beta")
        .await
        .expect("первый set_tag");
    catalog.set_tag(second, "beta").await.expect("перенос тега");

    // Act
    let result = catalog.set_tag(second, "beta").await;

    // Assert
    result.expect("повторный set_tag");
    assert_eq!(
        tags_of(catalog, repo, "serde").await,
        vec![pair("beta", 11)],
        "тег один и стоит на последней версии; повтор не меняет результат"
    );
}

async fn set_tag_of_unknown_version_is_not_found<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_repo(catalog, 1, "main").await;

    // Act
    let result = catalog.set_tag(unknown_version_id(), "beta").await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::NotFound)),
        "set_tag неизвестной версии даёт NotFound, получили {result:?}"
    );
}

async fn tags_belong_to_the_package_of_their_version<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let other_repo = ensure_repo(catalog, 2, "other").await;
    let id = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;
    publish(catalog, new_version(11, repo, "tokio", "1.0.0")).await;
    publish(catalog, new_version(12, other_repo, "serde", "1.0.0")).await;

    // Act
    let result = catalog.set_tag(id, "beta").await;

    // Assert
    result.expect("set_tag");
    assert_eq!(
        (
            tags_of(catalog, repo, "tokio").await,
            tags_of(catalog, other_repo, "serde").await
        ),
        (vec![], vec![]),
        "тег виден только у пакета своей версии и в своём репозитории"
    );
}

async fn remove_tag_removes_only_that_tag<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(
        catalog,
        tagged(new_version(10, repo, "serde", "1.0.0"), &["latest", "next"]),
    )
    .await;

    // Act
    let removed = catalog
        .remove_tag(repo, &name("serde"), "next")
        .await
        .expect("remove_tag");

    // Assert
    assert!(removed, "remove_tag существующего тега даёт true");
    assert_eq!(
        tags_of(catalog, repo, "serde").await,
        vec![pair("latest", 10)],
        "остальные теги остаются"
    );
}

async fn remove_absent_tag_is_false<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(
        catalog,
        tagged(new_version(10, repo, "serde", "1.0.0"), &["latest"]),
    )
    .await;

    // Act
    let removed = catalog.remove_tag(repo, &name("serde"), "next").await;

    // Assert
    assert!(
        matches!(removed, Ok(false)),
        "remove_tag отсутствующего тега даёт false, получили {removed:?}"
    );
}

async fn tags_of_unknown_package_is_empty<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;

    // Act
    let tags = catalog.tags(repo, &name("serde")).await;

    // Assert
    assert!(
        matches!(&tags, Ok(found) if found.is_empty()),
        "у неизвестного пакета тегов нет, получили {tags:?}"
    );
}

async fn set_deprecated_sets_message_visible_in_find_and_versions<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let id = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;

    // Act
    let result = catalog.set_deprecated(id, Some("use 2.x".to_owned())).await;

    // Assert
    result.expect("set_deprecated");
    let listed = catalog
        .versions(repo, &name("serde"))
        .await
        .expect("versions");
    assert_eq!(
        (
            find(catalog, repo, "serde", "1.0.0").await.deprecated,
            listed.into_iter().map(|v| v.deprecated).collect::<Vec<_>>()
        ),
        (Some("use 2.x".to_owned()), vec![Some("use 2.x".to_owned())]),
        "сообщение видно и в find_version, и в versions"
    );
}

async fn set_deprecated_none_clears_message_and_is_idempotent<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let id = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;
    catalog
        .set_deprecated(id, Some("use 2.x".to_owned()))
        .await
        .expect("set_deprecated в подготовке сценария");
    catalog
        .set_deprecated(id, None)
        .await
        .expect("первое снятие");

    // Act
    let result = catalog.set_deprecated(id, None).await;

    // Assert
    result.expect("повторное снятие");
    assert_eq!(
        find(catalog, repo, "serde", "1.0.0").await.deprecated,
        None,
        "None снимает сообщение; повтор не ошибка"
    );
}

async fn set_deprecated_of_unknown_id_is_not_found<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_repo(catalog, 1, "main").await;

    // Act
    let result = catalog
        .set_deprecated(unknown_version_id(), Some("x".to_owned()))
        .await;

    // Assert
    assert!(
        matches!(result, Err(CatalogError::NotFound)),
        "set_deprecated неизвестного id даёт NotFound, получили {result:?}"
    );
}
