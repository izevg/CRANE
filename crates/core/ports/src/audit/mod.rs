//! Порт журнала аудита.

use crane_domain::audit::AuditEvent;

/// Приёмник событий аудита. Синхронный и без ошибки: в v0.1 сбой журнала не валит запрос;
/// транзакционность с действием в v0.4 — забота unit of work, а не этого трейта.
pub trait AuditSink: Send + Sync {
    fn record(&self, event: AuditEvent);
}
