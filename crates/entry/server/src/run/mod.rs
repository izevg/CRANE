//! Роли процесса: запуск HTTP-серверов и миграции.

mod health;
mod proxy;

use std::future::IntoFuture;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use crane_app::registry::Registry;
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_http::{
    CargoSettings, FormatSettings, NpmSettings, ProxyRouters, StaticToken, admin_router, api_router,
};
use crane_memory::{MemoryBlobStore, MemoryCatalog};
use crane_objstore::ObjectBlobStore;
use crane_pg::PgCatalog;
use crane_ports::proxy::{NoUpstream, UpstreamNames};
use crane_ports::registry::{BlobStore, Catalog};
use crane_ports::system::IdGen;
use crane_siem::OcsfLogSink;
use futures_util::future::{self, Either};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tokio::runtime::Builder;

use crate::config::{Database, Settings, Storage};
use crate::namespaces;
use crate::system::{SystemClock, UuidV7Ids};
pub(crate) use health::healthcheck;

/// Дольше ждать соединение смысла нет: на старте сервер падает с понятной ошибкой,
/// а оркестратор перезапускает его, когда PostgreSQL поднимется.
const DB_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

pub(crate) type RunError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn serve(settings: Settings) -> Result<(), RunError> {
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(serve_async(settings))
}

/// Собирает роутеры API и admin над хранилищами из настроек. Отдельно от `serve`,
/// чтобы сборку (выбор хранилищ, подключение, репозитории) проверяли тесты без сокетов.
pub(crate) async fn routers(mut settings: Settings) -> Result<(Router, Router), RunError> {
    match &mut settings.storage {
        Storage::Memory => {
            assemble(
                MemoryCatalog::default(),
                MemoryBlobStore::default(),
                settings,
            )
            .await
        }
        Storage::Durable {
            database,
            blobs,
            s3_key,
        } => {
            // Подключаемся сразу: недоступная БД валит старт, а не первый запрос.
            let pool = connect(database).await?;
            let store = ObjectBlobStore::from_url(blobs, s3_key.take())
                .map_err(|error| format!("не удалось открыть CRANE_BLOBS: {error}"))?;
            assemble(PgCatalog::new(pool), store, settings).await
        }
    }
}

async fn connect(database: &Database) -> Result<PgPool, RunError> {
    PgPoolOptions::new()
        .acquire_timeout(DB_ACQUIRE_TIMEOUT)
        .connect_with(database.options().clone())
        .await
        .map_err(|error| {
            format!("не удалось подключиться к PostgreSQL (CRANE_DATABASE_URL): {error}").into()
        })
}

/// Настройки HTTP-слоя форматов: публичный адрес и предел публикации у всех общие.
fn format_settings(public_url: String, max_publish_bytes: usize) -> FormatSettings {
    FormatSettings {
        cargo: CargoSettings {
            public_url: public_url.clone(),
            max_publish_bytes,
        },
        npm: NpmSettings {
            public_url,
            max_publish_bytes,
        },
    }
}

async fn assemble<C: Catalog + 'static, B: BlobStore + 'static>(
    catalog: C,
    blobs: B,
    settings: Settings,
) -> Result<(Router, Router), RunError> {
    // Параметры подключения с паролем нужны были только для подключения.
    drop(settings.storage);
    let ids = Arc::new(UuidV7Ids);
    for (format, names) in settings.repos.of_kind(RepositoryKind::Hosted) {
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
    let formats = format_settings(settings.public_url, settings.max_publish_bytes);
    // Registry и движок делят один CAS: скачанные архивы лежат там же, где опубликованные.
    let blobs = Arc::new(blobs);
    // Движок и источники строятся до `Registry`: из того же движка берётся проверка, знает ли
    // публичный реестр имя, а каталог ещё нужен, чтобы зарегистрировать proxy-репозитории.
    let proxies = proxy::prepare_proxies(
        &catalog,
        ids.as_ref(),
        Arc::clone(&blobs),
        &settings.repos,
        settings.proxy,
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
    ));
    namespaces::reserve(&registry, ids.as_ref(), settings.namespaces).await?;
    let proxy_repos = proxies.map_or_else(ProxyRouters::default, |proxies| {
        proxies.mount(&registry, &formats)
    });
    let admin = admin_router(registry.clone());
    // Сырой токен живёт только до этой строки: дальше остаётся его хэш.
    let token = StaticToken::new(settings.token.expose().as_bytes(), settings.principal);
    drop(settings.token);
    let api = api_router(
        registry,
        formats,
        token,
        settings.trusted_proxies,
        audit,
        proxy_repos,
    );
    Ok((api, admin))
}

async fn serve_async(settings: Settings) -> Result<(), RunError> {
    let (http_port, admin_port) = (settings.http_port, settings.admin_port);
    let names = |format, kind| settings.repos.names(format, kind).to_vec();
    let repos = names(Format::Cargo, RepositoryKind::Hosted);
    let proxy_repos = names(Format::Cargo, RepositoryKind::Proxy);
    let npm_repos = names(Format::Npm, RepositoryKind::Hosted);
    let npm_proxy_repos = names(Format::Npm, RepositoryKind::Proxy);
    let (api, admin) = routers(settings).await?;

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

pub(crate) fn migrate(storage: Storage) -> Result<(), RunError> {
    match storage {
        Storage::Memory => {
            tracing::info!("хранилище в памяти: миграций нет");
            Ok(())
        }
        // Blob-хранилище миграциям не нужно: его URL уже разобран при чтении конфигурации.
        Storage::Durable { database, .. } => {
            let runtime = Builder::new_current_thread().enable_all().build()?;
            runtime.block_on(async {
                let pool = connect(&database).await?;
                drop(database);
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
#[path = "tests/stack.rs"]
mod stack_tests;
