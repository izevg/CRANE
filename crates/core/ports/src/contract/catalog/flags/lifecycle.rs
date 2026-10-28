use crane_domain::registry::VersionState;

use crate::contract::catalog::{ensure_repo, find, new_version, publish, unknown_version_id};
use crate::registry::{Catalog, CatalogError};

/// Сценарии группы; порядок вызовов сохраняет порядок общего прогона.
pub(super) async fn scenarios<C: Catalog>(new: &impl Fn() -> C) {
    transition_with_matching_from_changes_state(&new()).await;
    transition_with_stale_from_is_state_conflict_and_keeps_state(&new()).await;
    transition_of_unknown_id_is_not_found(&new()).await;
    set_yanked_true_sets_flag(&new()).await;
    set_yanked_true_twice_keeps_flag(&new()).await;
    set_yanked_false_clears_flag(&new()).await;
    set_yanked_of_unknown_id_is_not_found(&new()).await;
}

async fn transition_with_matching_from_changes_state<C: Catalog>(catalog: &C) {
    // Arrange
    let repo = ensure_repo(catalog, 1, "main").await;
    let id = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;

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
    let id = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;
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
    let id = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;

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
    let id = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;
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
    let id = publish(catalog, new_version(10, repo, "serde", "1.0.0"))
        .await
        .id;
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
