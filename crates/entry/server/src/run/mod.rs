//! Роли процесса: запуск HTTP-серверов и миграции.

mod health;

use std::future::IntoFuture;
use std::net::SocketAddr;
use std::sync::Arc;

use crane_app::registry::Registry;
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_http::{CargoSettings, StaticToken, admin_router, api_router};
use crane_memory::{MemoryBlobStore, MemoryCatalog};
use crane_ports::registry::Catalog;
use crane_ports::system::IdGen;
use crane_siem::OcsfLogSink;
use futures_util::future::{self, Either};
use tokio::net::TcpListener;
use tokio::runtime::Builder;

use crate::config::{Blobs, Catalogs, Settings, Storage};
use crate::system::{SystemClock, UuidV7Ids};
pub(crate) use health::healthcheck;

pub(crate) type RunError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn serve(settings: Settings) -> Result<(), RunError> {
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(serve_async(settings))
}

async fn serve_async(settings: Settings) -> Result<(), RunError> {
    let ids = Arc::new(UuidV7Ids);
    let (catalog, blobs) = match (settings.storage.catalog, settings.storage.blobs) {
        (Catalogs::Memory, Blobs::Memory) => (MemoryCatalog::default(), MemoryBlobStore::default()),
    };
    for name in &settings.cargo_repos {
        catalog
            .ensure_repository(Repository {
                id: RepositoryId::new(ids.generate()),
                format: Format::Cargo,
                kind: RepositoryKind::Hosted,
                name: name.clone(),
            })
            .await?;
    }
    // Один приёмник на use cases и на слой auth; в v0.4.0 действия записи уйдут в журнал PG.
    let audit = Arc::new(OcsfLogSink::new(Arc::new(SystemClock), ids.clone()));
    let registry = Arc::new(Registry::new(
        catalog,
        blobs,
        Arc::new(SystemClock),
        ids,
        audit.clone(),
    ));
    // Сырой токен живёт только до этой строки: дальше остаётся его хэш.
    let token = StaticToken::new(settings.token.expose(), settings.principal);
    drop(settings.token);
    let api = api_router(
        registry,
        CargoSettings {
            public_url: settings.public_url,
            max_publish_bytes: settings.max_publish_bytes,
        },
        token,
        settings.trusted_proxies,
        audit,
    );

    let api_listener = bind(settings.http_port).await?;
    let admin_listener = bind(settings.admin_port).await?;
    tracing::info!(
        http_port = settings.http_port,
        admin_port = settings.admin_port,
        repos = ?settings.cargo_repos,
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
    let admin_server = axum::serve(admin_listener, admin_router())
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

#[allow(
    clippy::unnecessary_wraps,
    reason = "сигнатура рассчитана на PostgreSQL-ветку, где миграция может упасть"
)]
pub(crate) fn migrate(storage: Storage) -> Result<(), RunError> {
    match (storage.catalog, storage.blobs) {
        (Catalogs::Memory, Blobs::Memory) => {
            tracing::info!("хранилище в памяти: миграций нет");
            Ok(())
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
