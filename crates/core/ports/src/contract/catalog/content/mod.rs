mod assets;
mod publish;

use crate::registry::Catalog;

/// Сценарии содержимого каталога: репозитории, версии и файлы версий.
pub(super) async fn scenarios<C: Catalog>(new: &impl Fn() -> C) {
    publish::scenarios(new).await;
    assets::scenarios(new).await;
}
