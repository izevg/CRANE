//! Знает ли публичный реестр имя: проверка перед первой публикацией в hosted-репозиторий.

use std::fmt;
use std::future::Future;
use std::pin::Pin;

use crane_app::proxy::{ProxyEngine, ProxyError};
use crane_domain::proxy::{MetadataPolicy, Origin, UpstreamPath};
use crane_domain::registry::{Format, PackageName};
use crane_format_cargo::{INDEX, index_path};
use crane_format_npm::{PACKUMENT, packument_path};
use crane_ports::proxy::{FetchLeases, Presence, ProxyCache, Upstream, UpstreamNames};
use crane_ports::registry::BlobStore;

/// Присутствие имени в публичном реестре по его документу (индекс crates.io, packument
/// registry.npmjs.org) через тот же движок, что и proxy-маршруты: общие кэш и singleflight,
/// та же политика egress. Формат без настроенного источника спрашивать не у кого.
pub struct EngineUpstreamNames<U, PC, L, B> {
    engine: ProxyEngine<U, PC, L, B>,
    cargo: Option<Origin>,
    npm: Option<Origin>,
}

// Вручную: derive потребовал бы `Debug` от портов движка.
impl<U, PC, L, B> fmt::Debug for EngineUpstreamNames<U, PC, L, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EngineUpstreamNames")
            .finish_non_exhaustive()
    }
}

impl<U, PC, L, B> EngineUpstreamNames<U, PC, L, B> {
    /// `cargo` и `npm` — источники документов первого proxy-репозитория формата; `None` — proxy
    /// формата не настроен.
    #[must_use]
    pub const fn new(
        engine: ProxyEngine<U, PC, L, B>,
        cargo: Option<Origin>,
        npm: Option<Origin>,
    ) -> Self {
        Self { engine, cargo, npm }
    }
}

impl<U, PC, L, B> EngineUpstreamNames<U, PC, L, B>
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
{
    /// Документ имени у upstream: путь строится из имени, а не приходит снаружи.
    async fn lookup(
        &self,
        origin: &Origin,
        path: Option<UpstreamPath>,
        policy: &MetadataPolicy,
    ) -> Presence {
        // Путь из имени, прошедшего грамматику формата, всегда разбирается; ветка нужна лишь типу.
        let Some(path) = path else {
            return Presence::Unknown;
        };
        // Устаревшая запись тоже доказывает, что имя публично; фоновая ревалидация ленивая
        // и брошенной ничего не стоит.
        match self.engine.metadata(origin, &path, policy).await {
            Ok(_) => Presence::Public,
            Err(ProxyError::NotFound) => Presence::Absent,
            Err(_) => Presence::Unknown,
        }
    }
}

impl<U, PC, L, B> UpstreamNames for EngineUpstreamNames<U, PC, L, B>
where
    U: Upstream + 'static,
    PC: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
{
    fn presence<'a>(
        &'a self,
        format: Format,
        name: &'a PackageName,
    ) -> Pin<Box<dyn Future<Output = Presence> + Send + 'a>> {
        Box::pin(async move {
            match (format, &self.cargo, &self.npm) {
                (Format::Cargo, Some(origin), _) => {
                    let path = UpstreamPath::parse(&index_path(name)).ok();
                    self.lookup(origin, path, &INDEX).await
                }
                (Format::Npm, _, Some(origin)) => {
                    let path = packument_path(name).ok();
                    self.lookup(origin, path, &PACKUMENT).await
                }
                // Нет источника для формата: proxy не настроен или реализации нет. Затенять нечего,
                // как и при `NoUpstream`.
                _ => Presence::Absent,
            }
        })
    }
}
