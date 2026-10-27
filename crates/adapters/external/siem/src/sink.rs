//! Приёмник, пишущий событие одной JSON-строкой журнала.

use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crane_domain::audit::AuditEvent;
use crane_ports::audit::AuditSink;
use crane_ports::system::{Clock, IdGen};

use crate::ocsf::{OcsfRecord, ocsf};

/// Журнал аудита в tracing, target `crane::audit`.
///
/// Имена полей — плоские пути OCSF (`actor.user.uid`); вложенный JSON собирает коллектор.
/// Необязательное значение, которого нет, не пишется вовсе (поле отсутствует в строке).
/// Приёмник сам ставит `metadata.uid` и `time`: эмиттеров события два (app и слой auth),
/// а часов и генератора id у http нет.
pub struct OcsfLogSink {
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGen>,
}

impl OcsfLogSink {
    #[must_use]
    pub fn new(clock: Arc<dyn Clock>, ids: Arc<dyn IdGen>) -> Self {
        Self { clock, ids }
    }
}

impl fmt::Debug for OcsfLogSink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OcsfLogSink").finish_non_exhaustive()
    }
}

/// Миллисекунды от эпохи; время до эпохи (часы сломаны) даёт 0, а не панику.
fn epoch_millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH).map_or(0, |elapsed| {
        u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
    })
}

impl AuditSink for OcsfLogSink {
    fn record(&self, event: AuditEvent) {
        let record = ocsf(&event, self.ids.generate(), epoch_millis(self.clock.now()));
        emit(&record);
    }
}

/// Цель `tracing` событий аудита. Сборка сервера держит её включённой при любом `RUST_LOG`.
pub const AUDIT_TARGET: &str = "crane::audit";

fn emit(r: &OcsfRecord) {
    let actor = r.actor.as_ref();
    let user = r.user.as_ref();
    tracing::info!(
        target: AUDIT_TARGET,
        class_uid = r.class_uid,
        category_uid = r.category_uid,
        activity_id = r.activity_id,
        activity_name = r.activity_name,
        type_uid = r.type_uid,
        status_id = r.status_id,
        status = r.status,
        status_detail = r.status_detail.as_deref(),
        severity_id = r.severity_id,
        metadata.uid = %r.event_id,
        metadata.correlation_uid = r.correlation_uid.as_str(),
        metadata.version = r.schema_version,
        metadata.product.name = r.product_name,
        metadata.product.version = r.product_version,
        time = r.time_ms,
        actor.user.uid = actor.and_then(|u| u.uid.as_deref()),
        actor.user.type_id = actor.map(|u| u.type_id),
        "actor.user.type" = actor.map(|u| u.type_name.as_str()),
        user.uid = user.and_then(|u| u.uid.as_deref()),
        user.type_id = user.map(|u| u.type_id),
        "user.type" = user.map(|u| u.type_name.as_str()),
        src_endpoint.ip = r.src_ip.as_deref(),
        http_request.user_agent = r.user_agent.as_deref(),
        api.operation = r.operation,
        resources.uid = r.resource_uid.as_deref(),
        "resources.type" = r.resource_type,
        unmapped.repository = r.repository.as_deref(),
        unmapped.trace_id = r.trace_id.as_deref(),
        "аудит"
    );
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn epoch_millis_counts_from_unix_epoch_and_clamps_earlier_times_to_zero() {
        // Arrange
        let cases = [
            UNIX_EPOCH,
            UNIX_EPOCH + Duration::from_millis(1_793_102_400_123),
            UNIX_EPOCH - Duration::from_secs(1),
        ];

        // Act
        let actual = cases.map(epoch_millis);

        // Assert
        assert_eq!(actual, [0, 1_793_102_400_123, 0]);
    }
}
