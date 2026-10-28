//! Namespace: зарезервированные паттерны имён и защита от dependency confusion.
//!
//! Имя внутреннее, если его покрывает namespace своего формата или пакет с таким
//! нормализованным именем уже есть в hosted-репозитории формата. Внутреннее имя никогда не
//! отдаётся из публичного upstream.
//!
//! Разрешение: побеждает самый специфичный из покрывающих namespace (точное имя, затем
//! самый длинный префикс) и для владения, и для приоритета hosted над proxy. Имя под
//! родительским namespace, из которого не вырезан вложенный, принадлежит владельцу родителя.

mod pattern;

use super::{Ecosystem, Format, PackageName, WorkspaceId};

pub use pattern::{NamespacePattern, PatternError};

/// Зарезервированный паттерн имён формата.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Namespace {
    pub format: Format,
    pub pattern: NamespacePattern,
    pub workspace: WorkspaceId,
}

/// Можно ли опубликовать имя в hosted-репозиторий экосистемы.
///
/// Для npm нельзя неименованное по scope: клиент маршрутизирует реестры только по scope,
/// поэтому безscope-имя нечем защитить от подмены из публичного реестра.
#[must_use]
pub fn hosted_name_allowed(eco: &dyn Ecosystem, name: &PackageName) -> bool {
    !eco.requires_scope() || name.normalized().starts_with('@')
}
