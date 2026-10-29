//! Роли процесса: запуск HTTP-серверов и миграции.

mod health;
mod phases;
mod proxy;

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use crane_app::registry::Registry;
use crane_domain::registry::{Repository, RepositoryId, RepositoryKind};
use crane_domain::settings::{Stored, Version};
use crane_http::{FormatSettings, StaticToken, admin_router, api_router};
use crane_memory::{MemoryBlobStore, MemoryCatalog, MemorySettings};
use crane_objstore::ObjectBlobStore;
use crane_pg::{PgCatalog, PgOnboarding, PgSettings, SettingsKey};
use crane_ports::proxy::{NoUpstream, UpstreamNames};
use crane_ports::registry::{BlobStore, Catalog};
use crane_ports::settings::SettingsStore;
use crane_ports::system::IdGen;
use crane_siem::OcsfLogSink;
use futures_util::future::{self, Either};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::runtime::Builder;

use crate::config::{
    Blobs, Boot, Config, ConfigError, ConfigSchema, Database, Repos, StoredLayer, settings_key,
};
use crate::ecosystem::ecosystem;
use crate::namespaces;
use crate::system::{SystemClock, UuidV7Ids};
pub(crate) use health::healthcheck;

/// Дольше ждать соединение смысла нет: на старте сервер падает с понятной ошибкой,
/// а оркестратор перезапускает его, когда PostgreSQL поднимется.
const DB_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) type RunError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn serve(boot: Boot) -> Result<(), RunError> {
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(phases::serve_async(boot))
}

/// Разобранная конфигурация и то, что нужно для остальной сборки.
pub(crate) struct Loaded {
    config: Config,
    /// Пул каталога; `None` при `memory`.
    pool: Option<PgPool>,
    settings: SettingsWiring,
}

/// Хранилище настроек: каталог в PostgreSQL или пустое в памяти.
enum SettingsBackend {
    Memory(MemorySettings),
    Postgres(PgSettings),
}

impl SettingsBackend {
    /// Строки БД; в памяти их нет.
    async fn list(&self) -> Result<BTreeMap<String, Stored>, RunError> {
        let rows = match self {
            Self::Memory(store) => store.list().await,
            Self::Postgres(store) => store.list().await,
        };
        Ok(rows?)
    }
}

/// Всё для `crane_app::settings::Settings::new`, кроме аудита (он появляется в `assemble`).
/// Ключ шифрования читается и потребляется один раз, в `load`.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "потребитель — HTTP настроек в B4a")
)]
pub(crate) struct SettingsWiring {
    store: SettingsBackend,
    schema: Arc<ConfigSchema>,
    /// Версии всех строк БД на старте; какие ключи учитывать, решает только app.
    started: BTreeMap<String, Version>,
}

impl SettingsWiring {
    /// Записывать есть куда: режим выводится из хранилища, отдельного флага нет.
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "потребитель — HTTP настроек в B4a")
    )]
    fn persistent(&self) -> bool {
        matches!(self.store, SettingsBackend::Postgres(_))
    }
}

/// Всё, что открывается один раз за жизнь процесса и переживает смену фаз `serve`.
pub(crate) struct Backend {
    pool: Option<PgPool>,
    settings: SettingsBackend,
    /// `None` при `memory`: первого запуска там нет, а мастеру нечего сохранять (20.2).
    onboarding: Option<Arc<PgOnboarding>>,
    schema: Arc<ConfigSchema>,
    persistent: bool,
}

impl Backend {
    /// Подключается к каталогу и открывает хранилища настроек и первого запуска. `Database`
    /// (с паролем) живёт только до подключения.
    pub(crate) async fn open(boot: &Boot) -> Result<Self, RunError> {
        let inherited = |name: &str| boot.var(name);
        let reader = |path: &Path| boot.read(path);
        let database = Database::from_env(&inherited, &reader)?;
        let persistent = database.persistent();
        let key = settings_key(&inherited, &reader, persistent)?;
        let opened = open_settings(&database, key).await?;
        drop(database);
        Ok(Self {
            pool: opened.pool,
            settings: opened.settings,
            onboarding: opened.onboarding,
            schema: Arc::new(ConfigSchema::new(boot.clone(), persistent)),
            persistent,
        })
    }

    /// Слои конфигурации: env + файл (`boot`), затем строки БД. Приоритет env > файл > БД > умолчание.
    /// Нечитаемый секрет не подменяется умолчанием: это такая же ошибка разбора, как неверное значение.
    fn parse(&self, boot: &Boot, rows: &BTreeMap<String, Stored>) -> Result<Config, ConfigError> {
        let inherited = |name: &str| boot.var(name);
        let reader = |path: &Path| boot.read(path);
        let (layer, ignored) = StoredLayer::new(&inherited, rows, &[])?;
        for (key, why) in &ignored {
            // Значений в журнале нет: только ключ и причина.
            tracing::warn!(key, reason = ?why, "строка настройки в БД не действует");
        }
        Config::from_layers(&inherited, &layer, &reader, self.persistent)
    }

    /// Хранилище первого запуска; его нет только при `memory`, где фаза мастера не выбирается.
    fn onboarding(&self) -> Result<&Arc<PgOnboarding>, RunError> {
        self.onboarding
            .as_ref()
            .ok_or_else(|| "первый запуск недоступен при CRANE_DATABASE_URL=memory".into())
    }

    fn into_loaded(self, config: Config, rows: &BTreeMap<String, Stored>) -> Loaded {
        let started = rows
            .iter()
            .map(|(key, row)| (key.clone(), row.version))
            .collect();
        Loaded {
            config,
            pool: self.pool,
            settings: SettingsWiring {
                store: self.settings,
                schema: self.schema,
                started,
            },
        }
    }
}

/// Слои и сборка за один вызов: для тестов, которым не нужен цикл фаз.
#[cfg(test)]
pub(crate) async fn load(boot: Boot) -> Result<Loaded, RunError> {
    let backend = Backend::open(&boot).await?;
    let rows = backend.settings.list().await?;
    let config = backend.parse(&boot, &rows)?;
    Ok(backend.into_loaded(config, &rows))
}

struct Opened {
    pool: Option<PgPool>,
    settings: SettingsBackend,
    onboarding: Option<Arc<PgOnboarding>>,
}

/// Хранилища настроек и первого запуска; в памяти строк нет, слой БД пуст, мастера нет.
async fn open_settings(database: &Database, key: Option<SettingsKey>) -> Result<Opened, RunError> {
    match database {
        Database::Memory => Ok(Opened {
            pool: None,
            settings: SettingsBackend::Memory(MemorySettings::new(Arc::new(SystemClock))),
            onboarding: None,
        }),
        Database::Postgres(options) => {
            // Подключаемся сразу: недоступная БД валит старт, а не первый запрос.
            let pool = connect(options).await?;
            let settings = PgSettings::new(pool.clone(), Arc::new(SystemClock), key);
            let onboarding = PgOnboarding::new(pool.clone(), Arc::new(SystemClock));
            Ok(Opened {
                pool: Some(pool),
                settings: SettingsBackend::Postgres(settings),
                onboarding: Some(Arc::new(onboarding)),
            })
        }
    }
}

/// Собирает роутеры API и admin над хранилищами из настроек. Отдельно от `serve`,
/// чтобы сборку (выбор хранилищ, репозитории) проверяли тесты без сокетов.
pub(crate) async fn routers(loaded: Loaded) -> Result<(Router, Router), RunError> {
    let Loaded {
        mut config,
        pool,
        // B4a соберёт здесь `crane_app::settings::Settings` и смонтирует HTTP настроек.
        settings: _wiring,
    } = loaded;
    match pool {
        None => assemble(MemoryCatalog::default(), MemoryBlobStore::default(), config).await,
        Some(pool) => {
            // Пару каталога и blob проверил `Blobs::from_env` по режиму каталога.
            let Blobs::Durable { url, s3_key } = &mut config.blobs else {
                return Err("постоянный каталог требует постоянного CRANE_BLOBS".into());
            };
            let store = ObjectBlobStore::from_url(url, s3_key.take())
                .map_err(|error| format!("не удалось открыть CRANE_BLOBS: {error}"))?;
            assemble(PgCatalog::new(pool), store, config).await
        }
    }
}

pub(crate) async fn connect(options: &PgConnectOptions) -> Result<PgPool, RunError> {
    PgPoolOptions::new()
        .acquire_timeout(DB_ACQUIRE_TIMEOUT)
        .connect_with(options.clone())
        .await
        .map_err(|error| {
            format!("не удалось подключиться к PostgreSQL (CRANE_DATABASE_URL): {error}").into()
        })
}

async fn assemble<C: Catalog + 'static, B: BlobStore + 'static>(
    catalog: C,
    blobs: B,
    config: Config,
) -> Result<(Router, Router), RunError> {
    // Ключ S3 нужен был только для открытия blob-хранилища.
    drop(config.blobs);
    let ids = Arc::new(UuidV7Ids);
    ensure_hosted_repositories(&catalog, ids.as_ref(), &config.repos).await?;
    // Публичный адрес и предел публикации у всех форматов общие.
    let formats = FormatSettings {
        public_url: config.public_url,
        max_publish_bytes: config.max_publish_bytes,
    };
    // Registry и движок делят один CAS: скачанные архивы лежат там же, где опубликованные.
    let blobs = Arc::new(blobs);
    // Движок и источники строятся до `Registry`: из того же движка берётся проверка, знает ли
    // публичный реестр имя, а каталог ещё нужен, чтобы зарегистрировать proxy-репозитории.
    let proxies = proxy::prepare_proxies(
        &catalog,
        ids.as_ref(),
        Arc::clone(&blobs),
        &config.repos,
        config.proxy,
    )
    .await?;
    let upstream: Arc<dyn UpstreamNames> = proxies
        .as_ref()
        .map_or_else(|| Arc::new(NoUpstream), proxy::Prepared::upstream_names);
    // Один приёмник на use cases и на слой auth; в v0.4.0 действия записи уйдут в журнал PG.
    let audit = Arc::new(OcsfLogSink::new(
        Arc::new(SystemClock),
        ids.clone(),
        ecosystem,
    ));
    let registry = Arc::new(Registry::new(
        catalog,
        blobs,
        Arc::new(SystemClock),
        ids.clone(),
        audit.clone(),
        upstream,
        ecosystem,
    ));
    namespaces::reserve(&registry, ids.as_ref(), config.namespaces).await?;
    let proxy_repos = proxies.map_or_else(
        || Ok(Vec::new()),
        |proxies| proxies.mount(&registry, &formats),
    )?;
    let admin = admin_router(registry.clone());
    // Сырой токен живёт только до этой строки: дальше остаётся его хэш.
    let token = StaticToken::new(config.token.expose().as_bytes(), config.principal);
    drop(config.token);
    let api = api_router(
        &registry,
        &formats,
        token,
        config.trusted_proxies,
        audit,
        proxy_repos,
    );
    Ok((api, admin))
}

/// Hosted-репозитории из настроек должны быть в каталоге до первой публикации.
async fn ensure_hosted_repositories<C: Catalog>(
    catalog: &C,
    ids: &dyn IdGen,
    repos: &Repos,
) -> Result<(), RunError> {
    for (format, names) in repos.of_kind(RepositoryKind::Hosted) {
        for name in names {
            catalog
                .ensure_repository(Repository {
                    id: RepositoryId::new(ids.generate()),
                    format,
                    kind: RepositoryKind::Hosted,
                    name: name.clone(),
                })
                .await?;
        }
    }
    Ok(())
}

pub(crate) fn migrate(database: Database) -> Result<(), RunError> {
    match database {
        Database::Memory => {
            tracing::info!("хранилище в памяти: миграций нет");
            Ok(())
        }
        Database::Postgres(options) => {
            let runtime = Builder::new_current_thread().enable_all().build()?;
            runtime.block_on(async {
                let pool = connect(&options).await?;
                drop(options);
                crane_pg::migrate(&pool)
                    .await
                    .map_err(|error| format!("миграции PostgreSQL не применены: {error}"))?;
                tracing::info!("миграции PostgreSQL применены");
                Ok(())
            })
        }
    }
}

/// SIGINT или SIGTERM: оркестратор останавливает контейнер именно SIGTERM.
async fn shutdown_signal() {
    let interrupt = Box::pin(async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "не удалось подписаться на SIGINT");
            future::pending::<()>().await;
        }
    });
    let terminate = Box::pin(terminate());
    match future::select(interrupt, terminate).await {
        Either::Left(_) => tracing::info!("получен SIGINT"),
        Either::Right(_) => tracing::info!("получен SIGTERM"),
    }
}

#[cfg(unix)]
async fn terminate() {
    use tokio::signal::unix::{SignalKind, signal};
    match signal(SignalKind::terminate()) {
        Ok(mut stream) => {
            stream.recv().await;
        }
        Err(error) => {
            tracing::error!(%error, "не удалось подписаться на SIGTERM");
            future::pending::<()>().await;
        }
    }
}

#[cfg(not(unix))]
async fn terminate() {
    future::pending::<()>().await;
}

#[cfg(test)]
#[path = "tests/settings.rs"]
mod settings_tests;

#[cfg(test)]
#[path = "tests/hosted.rs"]
mod hosted_tests;

#[cfg(test)]
#[path = "tests/stack.rs"]
mod stack;
