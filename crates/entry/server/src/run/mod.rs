//! Роли процесса: запуск HTTP-серверов и миграции.

mod health;

use std::future::IntoFuture;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use crane_app::registry::Registry;
use crane_domain::registry::{Format, Repository, RepositoryId, RepositoryKind};
use crane_http::{CargoSettings, StaticToken, admin_router, api_router};
use crane_memory::{MemoryBlobStore, MemoryCatalog};
use crane_objstore::ObjectBlobStore;
use crane_pg::PgCatalog;
use crane_ports::registry::{BlobStore, Catalog};
use crane_ports::system::IdGen;
use crane_siem::OcsfLogSink;
use futures_util::future::{self, Either};
use sqlx::PgPool;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tokio::net::TcpListener;
use tokio::runtime::Builder;

use crate::config::{Secret, Settings, Storage};
use crate::system::{SystemClock, UuidV7Ids};
pub(crate) use health::healthcheck;

/// Дольше ждать соединение смысла нет: на старте сервер падает с понятной ошибкой,
/// а оркестратор перезапускает его, когда PostgreSQL поднимется.
const DB_ACQUIRE_TIMEOUT: Duration = Duration::from_secs(5);

/// Имена переменных в сообщениях об ошибках: значения не показываются никогда.
const BLOBS_VAR: &str = "CRANE_BLOBS";
const DATABASE_VAR: &str = "CRANE_DATABASE_URL";

pub(crate) type RunError = Box<dyn std::error::Error + Send + Sync>;

pub(crate) fn serve(settings: Settings) -> Result<(), RunError> {
    let runtime = Builder::new_multi_thread().enable_all().build()?;
    runtime.block_on(serve_async(settings))
}

async fn serve_async(settings: Settings) -> Result<(), RunError> {
    match &settings.storage {
        Storage::Memory => {
            serve_with(
                MemoryCatalog::default(),
                MemoryBlobStore::default(),
                settings,
            )
            .await
        }
        Storage::Durable { database, blobs } => {
            // Подключаемся сразу: недоступная БД валит старт, а не первый запрос.
            let pool = connect(database).await?;
            let store = ObjectBlobStore::from_url(blobs)
                .map_err(|error| format!("не удалось открыть {BLOBS_VAR}: {error}"))?;
            serve_with(PgCatalog::new(pool), store, settings).await
        }
    }
}

/// Параметры подключения без текста ошибки разбора: он может процитировать URL с паролем.
fn connect_options(url: &str) -> Result<PgConnectOptions, RunError> {
    PgConnectOptions::from_str(url)
        .map_err(|_| format!("{DATABASE_VAR} не разобран как URL PostgreSQL").into())
}

async fn connect(database: &Secret) -> Result<PgPool, RunError> {
    let options = connect_options(database.expose())?;
    PgPoolOptions::new()
        .acquire_timeout(DB_ACQUIRE_TIMEOUT)
        .connect_with(options)
        .await
        .map_err(|error| {
            format!("не удалось подключиться к PostgreSQL ({DATABASE_VAR}): {error}").into()
        })
}

async fn serve_with<C: Catalog + 'static, B: BlobStore + 'static>(
    catalog: C,
    blobs: B,
    settings: Settings,
) -> Result<(), RunError> {
    // Адрес базы с паролем нужен был только для подключения.
    drop(settings.storage);
    let ids = Arc::new(UuidV7Ids);
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
    let admin = admin_router(registry.clone());
    // Сырой токен живёт только до этой строки: дальше остаётся его хэш.
    let token = StaticToken::new(settings.token.expose().as_bytes(), settings.principal);
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
mod tests {
    use super::*;

    #[test]
    fn unparsable_database_url_error_never_contains_the_password() {
        // Arrange
        // sqlx цитирует значение `sslmode` в тексте ошибки: пароль в нём показал бы, что маскировка снята.
        let urls = [
            "postgres://u:pw@h/db?sslmode=SECRETPW",
            "postgres://u:SECRETPW@h:notaport/db",
        ];

        // Act
        let messages = urls.map(|url| connect_options(url).err().map(|error| error.to_string()));

        // Assert
        let leaked = messages.map(|message| message.is_none_or(|text| text.contains("SECRETPW")));
        assert_eq!(leaked, [false; 2]);
    }
}
