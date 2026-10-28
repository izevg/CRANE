//! Применение `CRANE_NAMESPACES` при старте: отдельный шаг запуска, не часть сборки роутеров.

use crane_app::registry::{Registry, RegistryError};
use crane_domain::access::{Caller, Principal, PrincipalKind, RequestContext, RequestId};
use crane_domain::registry::Namespace;
use crane_ports::registry::{BlobStore, Catalog};
use crane_ports::system::IdGen;

use crate::run::RunError;

/// Применяет `CRANE_NAMESPACES` идемпотентно. Чужой владелец уже занятого шаблона валит старт:
/// владение молча не переходит. Любая ошибка (в том числе каталога) тоже останавливает запуск.
pub(crate) async fn reserve<C: Catalog, B: BlobStore>(
    registry: &Registry<C, B>,
    ids: &dyn IdGen,
    namespaces: Vec<Namespace>,
) -> Result<(), RunError> {
    // Каждое резервирование, в том числе из конфигурации, аудируется с актором и контекстом
    // запроса (требование владельца); контекст один на старт, как и сам запуск.
    let caller = Caller {
        principal: Principal {
            id: "crane:config".to_owned(),
            kind: PrincipalKind::Workload,
        },
        request: RequestContext {
            request_id: RequestId::from(ids.generate()),
            trace_id: None,
            client_ip: None,
            user_agent: None,
        },
    };
    for namespace in namespaces {
        let shown = namespace.pattern.as_str().to_owned();
        registry
            .reserve(&caller, namespace)
            .await
            .map_err(|error| match error {
                RegistryError::NamespaceConflict => format!(
                    "namespace {shown} уже принадлежит другому workspace; \
                     владение не передаётся через конфигурацию"
                ),
                other => format!("не удалось применить CRANE_NAMESPACES ({shown}): {other}"),
            })?;
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap - это падение теста"
)]
mod tests {
    use std::sync::Arc;
    use std::time::SystemTime;

    use crane_domain::audit::AuditAction;
    use crane_domain::registry::{Format, NamespacePattern, WorkspaceId};
    use crane_memory::{ManualClock, MemoryAudit, MemoryBlobStore, MemoryCatalog, SequentialIdGen};
    use crane_ports::proxy::NoUpstream;
    use tokio::runtime::Builder;
    use uuid::Uuid;

    use super::*;

    struct Fixture {
        registry: Registry<MemoryCatalog, MemoryBlobStore>,
        audit: Arc<MemoryAudit>,
        ids: SequentialIdGen,
    }

    fn fixture() -> Fixture {
        let audit = Arc::new(MemoryAudit::default());
        let registry = Registry::new(
            MemoryCatalog::default(),
            MemoryBlobStore::default(),
            Arc::new(ManualClock::new(SystemTime::UNIX_EPOCH)),
            Arc::new(SequentialIdGen::default()),
            audit.clone(),
            Arc::new(NoUpstream),
        );
        Fixture {
            registry,
            audit,
            ids: SequentialIdGen::default(),
        }
    }

    fn entry(pattern: &str, workspace: WorkspaceId) -> Namespace {
        Namespace {
            format: Format::Cargo,
            pattern: NamespacePattern::parse(Format::Cargo, pattern).unwrap(),
            workspace,
        }
    }

    fn other_workspace() -> WorkspaceId {
        WorkspaceId::new(Uuid::from_u128(2))
    }

    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    #[test]
    fn every_applied_entry_is_audited_with_the_config_actor() {
        // Arrange
        let fixture = fixture();
        let entries = vec![
            entry("acme-*", WorkspaceId::DEFAULT),
            entry("zeta-*", other_workspace()),
        ];

        // Act
        let result = block_on(reserve(&fixture.registry, &fixture.ids, entries));

        // Assert
        assert!(result.is_ok());
        let reserved: Vec<Option<String>> = fixture
            .audit
            .events()
            .iter()
            .filter(|event| matches!(event.action, AuditAction::Reserve(_)))
            .map(|event| event.actor.as_ref().map(|actor| actor.id.clone()))
            .collect();
        let actor = || Some("crane:config".to_owned());
        assert_eq!(reserved, [actor(), actor()]);
    }

    #[test]
    fn reapplying_the_same_entries_succeeds() {
        // Arrange
        let fixture = fixture();
        let entries = || vec![entry("acme-*", WorkspaceId::DEFAULT)];
        block_on(reserve(&fixture.registry, &fixture.ids, entries())).unwrap();

        // Act
        let result = block_on(reserve(&fixture.registry, &fixture.ids, entries()));

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn pattern_owned_by_another_workspace_fails_startup_without_transfer() {
        // Arrange
        let fixture = fixture();
        block_on(reserve(
            &fixture.registry,
            &fixture.ids,
            vec![entry("acme-*", WorkspaceId::DEFAULT)],
        ))
        .unwrap();

        // Act
        let result = block_on(reserve(
            &fixture.registry,
            &fixture.ids,
            vec![entry("acme-*", other_workspace())],
        ));

        // Assert
        let message = result.err().map(|error| error.to_string()).unwrap();
        assert!(
            message.contains("acme_* уже принадлежит другому workspace"),
            "{message}"
        );
    }
}
