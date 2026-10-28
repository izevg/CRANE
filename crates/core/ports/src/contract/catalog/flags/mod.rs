mod lifecycle;
mod tags;

use crate::registry::Catalog;

/// Сценарии состояния версий: жизненный цикл, yank, устаревание и теги.
pub(super) async fn scenarios<C: Catalog>(new: &impl Fn() -> C) {
    lifecycle::scenarios(new).await;
    tags::scenarios(new).await;
}
