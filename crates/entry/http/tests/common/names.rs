//! Фикстуры защиты имён: резервирование «как из конфигурации» и публичный реестр с заданным ответом.

use std::future::{Future, ready};
use std::pin::Pin;

use crane_domain::access::{Caller, Principal, PrincipalKind, RequestContext, RequestId};
use crane_domain::registry::{Format, Namespace, NamespacePattern, PackageName, WorkspaceId};
use crane_ports::proxy::{Presence, UpstreamNames};

use super::Api;

/// Публичный реестр, который на любое имя отвечает одним и тем же.
pub(crate) struct FixedPresence(pub(crate) Presence);

impl UpstreamNames for FixedPresence {
    fn presence<'a>(
        &'a self,
        _format: Format,
        _name: &'a PackageName,
    ) -> Pin<Box<dyn Future<Output = Presence> + Send + 'a>> {
        Box::pin(ready(self.0))
    }
}

impl Api {
    /// Резервирует namespace так, как это делает конфигурация сервера.
    pub(crate) fn reserve(&self, format: Format, pattern: &str) {
        let caller = Caller {
            principal: Principal {
                id: "crane:config".to_owned(),
                kind: PrincipalKind::Workload,
            },
            request: RequestContext {
                request_id: RequestId::parse("seed").unwrap(),
                trace_id: None,
                client_ip: None,
                user_agent: None,
            },
        };
        let namespace = Namespace {
            format,
            pattern: NamespacePattern::parse(format, pattern).unwrap(),
            workspace: WorkspaceId::DEFAULT,
        };
        self.runtime
            .block_on(self.registry.reserve(&caller, namespace))
            .unwrap();
    }
}
