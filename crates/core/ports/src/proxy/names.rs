use std::future::Future;
use std::pin::Pin;

use crane_domain::registry::{Format, PackageName};

/// Что публичный upstream формата знает об имени.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// Имя опубликовано в публичном реестре.
    Public,
    /// Upstream ответил, что имени нет.
    Absent,
    /// Upstream не смог ответить (сеть, таймаут, 5xx). Для защиты это не «нет»:
    /// решающий use case отказывает, а не гадает.
    Unknown,
}

/// Знает ли публичный upstream формата это имя.
///
/// В отличие от остальных асинхронных портов dyn-совместим, а future упакован: реализация
/// выбирается при сборке (proxy формата есть или нет), как `Clock`, и `Registry` не обобщается
/// по ещё одному параметру.
pub trait UpstreamNames: Send + Sync {
    fn presence<'a>(
        &'a self,
        format: Format,
        name: &'a PackageName,
    ) -> Pin<Box<dyn Future<Output = Presence> + Send + 'a>>;
}

/// Формату без proxy затенять нечего: имя всегда `Absent`.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoUpstream;

impl UpstreamNames for NoUpstream {
    fn presence<'a>(
        &'a self,
        _format: Format,
        _name: &'a PackageName,
    ) -> Pin<Box<dyn Future<Output = Presence> + Send + 'a>> {
        Box::pin(std::future::ready(Presence::Absent))
    }
}
