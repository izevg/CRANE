//! Роли процесса: запуск HTTP-серверов и миграции.

mod health;
mod proxy;

use std::collections::BTreeMap;
use std::future::IntoFuture;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use crane_app::registry::Registry;
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_domain::settings::{Stored, Version};
use crane_http::{FormatSettings, StaticToken, admin_router, api_router};
use crane_memory::{MemoryBlobStore, MemoryCatalog, MemorySettings};
use crane_objstore::ObjectBlobStore;
use crane_pg::{PgCatalog, PgSettings, SettingsKey};
use crane_ports::proxy::{NoUpstream, UpstreamNames};
use crane_ports::registry::{BlobStore, Catalog};
use crane_ports::settings::SettingsStore;
use crane_ports::system::IdGen;
use crane_siem::OcsfLogSink;
use futures_util::future::{self, Either};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::net::TcpListener;
use tokio::runtime::Builder;

use crate::config::{Blobs, Boot, Config, ConfigSchema, Database, StoredLayer, settings_key};
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
    runtime.block_on(serve_async(boot))
}

/// Разобранная конфигурация и то, что нужно для остальной сборки.
pub(crate) struct Loaded {
    config: Config,
    /// Пул каталога; `None` при `memory`.
    pool: Option<PgPool>,
    settings: SettingsWiring,
}

/// Хранилище настроек: каталог в PostgreSQL или пустое в памяти.
#[expect(dead_code, reason = "потребитель — HTTP настроек в B4a")]
enum SettingsBackend {
    Memory(MemorySettings),
    Postgres(PgSettings),
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

/// Слои конфигурации: env + файл (`boot`), затем строки БД. Приоритет env > файл > БД > умолчание.
/// Любая ошибка разбора валит старт: нечитаемый секрет не подменяется умолчанием.
pub(crate) async fn load(boot: Boot) -> Result<Loaded, RunError> {
    let inherited = |name: &str| boot.var(name);
    let reader = |path: &Path| boot.read(path);
    let database = Database::from_env(&inherited, &reader)?;
    let key = settings_key(&inherited, &reader, database.persistent())?;
    let (pool, store, rows) = open_settings(&database, key).await?;
    let (layer, ignored) = StoredLayer::new(&inherited, &rows, &[])?;
    for (key, why) in &ignored {
        // Значений в журнале нет: только ключ и причина.
        tracing::warn!(key, reason = ?why, "строка настройки в БД не действует");
    }
    let config = Config::from_layers(&inherited, &layer, &reader, database.persistent())?;
    let started = rows
        .iter()
        .map(|(key, row)| (key.clone(), row.version))
        .collect();
    let schema = Arc::new(ConfigSchema::new(boot.clone(), database.persistent()));
    Ok(Loaded {
        config,
        pool,
        settings: SettingsWiring {
            store,
            schema,
            started,
        },
    })
}

/// Хранилище настроек и его строки; в памяти строк нет, слой БД пуст.
async fn open_settings(
    database: &Database,
    key: Option<SettingsKey>,
) -> Result<(Option<PgPool>, SettingsBackend, BTreeMap<String, Stored>), RunError> {
    match database {
        Database::Memory => {
            let store = MemorySettings::new(Arc::new(SystemClock));
            Ok((None, SettingsBackend::Memory(store), BTreeMap::new()))
        }
        Database::Postgres(options) => {
            // Подключаемся сразу: недоступная БД валит старт, а не первый запрос.
            let pool = connect(options).await?;
            let store = PgSettings::new(pool.clone(), Arc::new(SystemClock), key);
            let rows = store.list().await?;
            Ok((Some(pool), SettingsBackend::Postgres(store), rows))
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

async fn connect(options: &PgConnectOptions) -> Result<PgPool, RunError> {
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
    for (format, names) in config.repos.of_kind(RepositoryKind::Hosted) {
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
    let audit = Arc::new(OcsfLogSink::new(Arc::new(SystemClock), ids.clone()));
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

async fn serve_async(boot: Boot) -> Result<(), RunError> {
    let loaded = load(boot).await?;
    let (http_port, admin_port) = (loaded.config.http_port, loaded.config.admin_port);
    let names = |format, kind| loaded.config.repos.names(format, kind).to_vec();
    let repos = names(Format::Cargo, RepositoryKind::Hosted);
    let proxy_repos = names(Format::Cargo, RepositoryKind::Proxy);
    let npm_repos = names(Format::Npm, RepositoryKind::Hosted);
    let npm_proxy_repos = names(Format::Npm, RepositoryKind::Proxy);
    let (api, admin) = routers(loaded).await?;

    let api_listener = bind(http_port).await?;
    let admin_listener = bind(admin_port).await?;
    tracing::info!(
        http_port,
        admin_port,
        repos = ?repos,
        proxy_repos = ?proxy_repos,
        npm_repos = ?npm_repos,
        npm_proxy_repos = ?npm_proxy_repos,
        "сервер запущен"
    );

    // Каждый сервер ждёт свою копию сигнала: tokio::signal раздаёт его всем подписчикам.
    let api_server = axum::serve(
        api_listener,
        // Адрес пира нужен слою контекста запроса (доверие к X-Forwarded-For).
        api.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .into_future();
    let admin_server = axum::serve(admin_listener, admin)
        .with_graceful_shutdown(shutdown_signal())
        .into_future();
    future::try_join(api_server, admin_server).await?;
    tracing::info!("сервер остановлен");
    Ok(())
}

async fn bind(port: u16) -> Result<TcpListener, RunError> {
    TcpListener::bind(("0.0.0.0", port))
        .await
        .map_err(|error| format!("не удалось занять порт {port}: {error}").into())
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
#[path = "tests/stack.rs"]
mod stack;
