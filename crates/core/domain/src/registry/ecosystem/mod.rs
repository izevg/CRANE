//! Как экосистема пишет имена и версии: трейт `Ecosystem` и ключ порядка версий.
//!
//! Трейт лежит в `domain`, потому что это чистая политика без IO, а `NamespacePattern` обязан
//! её вызывать. Реализации живут в `formats`, выбор по `Format` — одна функция в `entry`.

mod order;

use super::{Format, NameError, PackageName, Version, VersionError};

pub use order::{OrderKey, OrderKeyBuilder};

/// Как экосистема пишет имена и версии. Без IO; реализации — в formats, выбор по `Format` — в entry.
pub trait Ecosystem: Send + Sync {
    fn format(&self) -> Format;

    /// Проверяет грамматику имени и строит нормализованную и написанную формы.
    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError>;

    /// Проверяет грамматику версии и строит каноническую форму, ключ порядка и признак pre-release.
    fn parse_version(&self, raw: &str) -> Result<Version, VersionError>;

    /// Нормализация базы префикса namespace (база — не всегда имя: groupId Maven).
    fn normalize(&self, raw: &str) -> String;

    /// Разделитель нормализованной формы: по нему префикс обрывается на границе сегмента.
    fn separator(&self) -> char;

    /// Разделители, которыми клиент может записать границу перед `*`.
    fn accepted_separators(&self) -> &'static [char];

    /// npm: hosted-имя без scope недопустимо.
    fn requires_scope(&self) -> bool {
        false
    }
}

/// Выбор реализации по тегу формата; настоящую функцию подставляет `entry`, чтобы `app`
/// не зависел от `formats`.
pub type Ecosystems = fn(Format) -> &'static dyn Ecosystem;
