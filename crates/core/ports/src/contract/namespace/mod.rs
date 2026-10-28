//! Контракт namespace-части `Catalog`: поиск покрывающего namespace, занятость имени
//! hosted-репозиторием и идемпотентное резервирование. Отдельно от `catalog_contract`:
//! эти методы нужны защите от dependency confusion, а не жизненному циклу версий.

use crane_domain::registry::{
    Format, Namespace, Repository, RepositoryId, RepositoryKind, WorkspaceId,
};
use uuid::Uuid;

use super::catalog::{ensure_repo, new_version, publish, repository};
use super::fixture::{candidates, name, pattern};
use crate::registry::Catalog;

/// Проверяет namespace-методы реализации `Catalog`; `new` создаёт чистый каталог для сценария.
pub async fn namespace_contract<C: Catalog>(new: impl Fn() -> C) {
    namespace_for_prefers_exact_pattern_over_prefix(&new()).await;
    namespace_for_nested_prefix_picks_longest_for_inner_name(&new()).await;
    namespace_for_nested_prefix_picks_outer_for_other_name(&new()).await;
    namespace_for_name_sharing_only_leading_letters_is_none(&new()).await;
    namespace_for_other_format_is_none(&new()).await;
    namespace_for_follows_the_order_of_candidates(&new()).await;
    namespace_for_without_candidates_is_none(&new()).await;
    hosted_exists_is_true_for_package_in_hosted_repository(&new()).await;
    hosted_exists_is_true_for_other_spelling_of_name(&new()).await;
    hosted_exists_is_false_for_package_only_in_proxy_repository(&new()).await;
    hosted_exists_is_false_for_other_format(&new()).await;
    hosted_exists_is_false_for_unknown_name(&new()).await;
    ensure_namespace_returns_new_namespace_as_given(&new()).await;
    ensure_namespace_with_same_pattern_returns_stored_owner(&new()).await;
}

fn workspace(id: u128) -> WorkspaceId {
    WorkspaceId::new(Uuid::from_u128(id))
}

fn namespace(format: Format, raw_pattern: &str, owner: u128) -> Namespace {
    Namespace {
        format,
        pattern: pattern(raw_pattern),
        workspace: workspace(owner),
    }
}

async fn ensure_ns<C: Catalog>(catalog: &C, namespace: &Namespace) {
    catalog
        .ensure_namespace(namespace.clone())
        .await
        .expect("ensure_namespace в подготовке сценария");
}

/// Репозиторий заданного вида: `ensure_repo` создаёт только hosted.
async fn ensure_repo_of_kind<C: Catalog>(
    catalog: &C,
    id: u128,
    repo_name: &str,
    kind: RepositoryKind,
) -> RepositoryId {
    catalog
        .ensure_repository(Repository {
            kind,
            ..repository(id, repo_name)
        })
        .await
        .expect("ensure_repository в подготовке сценария")
        .id
}

async fn namespace_for_prefers_exact_pattern_over_prefix<C: Catalog>(catalog: &C) {
    // Arrange
    let prefix = namespace(Format::Cargo, "acme-*", 100);
    let exact = namespace(Format::Cargo, "acme-core", 200);
    ensure_ns(catalog, &prefix).await;
    ensure_ns(catalog, &exact).await;

    // Act
    let result = catalog
        .namespace_for(Format::Cargo, &candidates(&name("acme_core")))
        .await;

    // Assert
    assert_eq!(
        result.expect("namespace_for"),
        Some(exact),
        "namespace_for: точное имя специфичнее префикса"
    );
}

async fn namespace_for_nested_prefix_picks_longest_for_inner_name<C: Catalog>(catalog: &C) {
    // Arrange
    let outer = namespace(Format::Cargo, "acme-*", 100);
    let inner = namespace(Format::Cargo, "acme-payments-*", 200);
    ensure_ns(catalog, &outer).await;
    ensure_ns(catalog, &inner).await;

    // Act
    let result = catalog
        .namespace_for(Format::Cargo, &candidates(&name("acme_payments_core")))
        .await;

    // Assert
    assert_eq!(
        result.expect("namespace_for"),
        Some(inner),
        "namespace_for: из вложенных префиксов побеждает самый длинный (владелец вложенного)"
    );
}

async fn namespace_for_nested_prefix_picks_outer_for_other_name<C: Catalog>(catalog: &C) {
    // Arrange
    let outer = namespace(Format::Cargo, "acme-*", 100);
    let inner = namespace(Format::Cargo, "acme-payments-*", 200);
    ensure_ns(catalog, &outer).await;
    ensure_ns(catalog, &inner).await;

    // Act
    let result = catalog
        .namespace_for(Format::Cargo, &candidates(&name("acme_billing")))
        .await;

    // Assert
    assert_eq!(
        result.expect("namespace_for"),
        Some(outer),
        "namespace_for: имя вне вложенного префикса принадлежит владельцу внешнего"
    );
}

async fn namespace_for_name_sharing_only_leading_letters_is_none<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_ns(catalog, &namespace(Format::Cargo, "acme-*", 100)).await;

    // Act
    let result = catalog
        .namespace_for(Format::Cargo, &candidates(&name("acmecorp")))
        .await;

    // Assert
    assert_eq!(
        result.expect("namespace_for"),
        None,
        "namespace_for: префикс acme_* кончается разделителем и не покрывает acmecorp"
    );
}

async fn namespace_for_other_format_is_none<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_ns(catalog, &namespace(Format::Cargo, "acme-core", 100)).await;

    // Act
    let result = catalog
        .namespace_for(Format::PyPi, &candidates(&name("acme_core")))
        .await;

    // Assert
    assert_eq!(
        result.expect("namespace_for"),
        None,
        "namespace_for: namespace другого формата имя не покрывает"
    );
}

async fn namespace_for_follows_the_order_of_candidates<C: Catalog>(catalog: &C) {
    // Arrange
    let prefix = namespace(Format::Cargo, "acme-*", 100);
    let exact = namespace(Format::Cargo, "acme-core", 200);
    ensure_ns(catalog, &prefix).await;
    ensure_ns(catalog, &exact).await;
    let reversed = [pattern("acme-*"), pattern("acme-core")];

    // Act
    let result = catalog.namespace_for(Format::Cargo, &reversed).await;

    // Assert
    assert_eq!(
        result.expect("namespace_for"),
        Some(prefix),
        "namespace_for: порядок кандидатов задаёт вызывающий, первый сохранённый побеждает"
    );
}

async fn namespace_for_without_candidates_is_none<C: Catalog>(catalog: &C) {
    // Arrange
    ensure_ns(catalog, &namespace(Format::Cargo, "acme-*", 100)).await;

    // Act
    let result = catalog.namespace_for(Format::Cargo, &[]).await;

    // Assert
    assert_eq!(
        result.expect("namespace_for"),
        None,
        "namespace_for: без кандидатов покрытия нет"
    );
}

async fn hosted_exists_is_true_for_package_in_hosted_repository<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "serde", "1.0.0")).await;

    // Act
    let result = catalog.hosted_exists(Format::Cargo, &name("serde")).await;

    // Assert
    assert!(
        result.expect("hosted_exists"),
        "hosted_exists: пакет в hosted-репозитории формата найден"
    );
}

async fn hosted_exists_is_true_for_other_spelling_of_name<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "Foo-Bar", "1.0.0")).await;

    // Act
    let result = catalog.hosted_exists(Format::Cargo, &name("foo_bar")).await;

    // Assert
    assert!(
        result.expect("hosted_exists"),
        "hosted_exists сравнивает нормализованные имена: Foo-Bar и foo_bar - одно имя"
    );
}

async fn hosted_exists_is_false_for_package_only_in_proxy_repository<C: Catalog>(catalog: &C) {
    // Arrange
    let proxy = ensure_repo_of_kind(catalog, 1, "mirror", RepositoryKind::Proxy).await;
    publish(catalog, new_version(10, proxy, "serde", "1.0.0")).await;

    // Act
    let result = catalog.hosted_exists(Format::Cargo, &name("serde")).await;

    // Assert
    assert!(
        !result.expect("hosted_exists"),
        "hosted_exists: закэшированный из upstream пакет в proxy-репозитории не делает имя внутренним"
    );
}

async fn hosted_exists_is_false_for_other_format<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "serde", "1.0.0")).await;

    // Act
    let result = catalog.hosted_exists(Format::Npm, &name("serde")).await;

    // Assert
    assert!(
        !result.expect("hosted_exists"),
        "hosted_exists: пакет другого формата имя не занимает"
    );
}

async fn hosted_exists_is_false_for_unknown_name<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    publish(catalog, new_version(10, repo, "serde", "1.0.0")).await;

    // Act
    let result = catalog.hosted_exists(Format::Cargo, &name("tokio")).await;

    // Assert
    assert!(
        !result.expect("hosted_exists"),
        "hosted_exists: неизвестное имя даёт false"
    );
}

async fn ensure_namespace_returns_new_namespace_as_given<C: Catalog>(catalog: &C) {
    // Arrange
    let given = namespace(Format::Cargo, "acme-*", 100);

    // Act
    let result = catalog.ensure_namespace(given.clone()).await;

    // Assert
    assert_eq!(
        result.expect("ensure_namespace"),
        given,
        "ensure_namespace: новый namespace возвращается как передан"
    );
}

async fn ensure_namespace_with_same_pattern_returns_stored_owner<C: Catalog>(catalog: &C) {
    // Arrange
    let first = namespace(Format::Cargo, "acme-*", 100);
    ensure_ns(catalog, &first).await;

    // Act
    let result = catalog
        .ensure_namespace(namespace(Format::Cargo, "acme-*", 200))
        .await;

    // Assert
    assert_eq!(
        result.expect("повторный ensure_namespace"),
        first,
        "ensure_namespace идемпотентен: при том же (format, pattern) побеждает сохранённый владелец"
    );
}
