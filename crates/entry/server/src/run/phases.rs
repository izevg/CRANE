//! Цикл фаз `serve`: мастер первого запуска, затем рабочий режим в том же процессе (20.2).

use std::collections::BTreeMap;
use std::future::{Future, IntoFuture};
use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use crane_domain::registry::{Format, RepositoryKind};
use crane_domain::settings::Stored;
use crane_http::{wizard_admin_router, wizard_router};
use crane_ports::settings::OnboardingStore;
use crane_siem::OcsfLogSink;
use futures_util::future::{self, Either, FutureExt};
use tokio::net::TcpListener;

use super::{Backend, RunError, routers, shutdown_signal};
use crate::config::{Boot, Config, admin_port, http_port};
use crate::ecosystem::ecosystem;
use crate::onboarding::{
    Journal, Row, Source, Step, WIZARD_POLL, decide, preconfigure, refresh_token, start_wizard,
    stored_token_set, until_completed,
};
use crate::system::{SystemClock, UuidV7Ids};

/// Что делает процесс после очередного прохода цикла.
enum Next {
    /// Состояние изменилось (строка `onboarding` появилась): прочитать слои заново.
    Again,
    Wizard,
    Ready(Box<Config>, BTreeMap<String, Stored>),
}

/// Чем закончилась фаза мастера.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stopped {
    /// SIGINT или SIGTERM: процесс завершается.
    Signal,
    /// Мастер завершён: слои перечитываются, процесс входит в рабочий режим.
    Completed,
}

struct Phases {
    boot: Boot,
    backend: Backend,
    journal: Journal,
}

pub(super) async fn serve_async(boot: Boot) -> Result<(), RunError> {
    let backend = Backend::open(&boot).await?;
    let ids = Arc::new(UuidV7Ids);
    let sink = Arc::new(OcsfLogSink::new(
        Arc::new(SystemClock),
        ids.clone(),
        ecosystem,
    ));
    let phases = Phases {
        boot,
        backend,
        journal: Journal::new(sink, ids),
    };
    loop {
        match phases.advance().await? {
            Next::Again => {}
            Next::Wizard => {
                if phases.serve_wizard().await? == Stopped::Signal {
                    return Ok(());
                }
            }
            Next::Ready(config, rows) => return phases.serve_ready(*config, &rows).await,
        }
    }
}

impl Phases {
    /// Один проход: слои, состояние `onboarding`, решение (`decide`) и его первый шаг.
    /// Перечитывается только `list()`: файл TOML и окружение за жизнь процесса не меняются.
    async fn advance(&self) -> Result<Next, RunError> {
        let rows = self.backend.settings.list().await?;
        let parsed = self.backend.parse(&self.boot, &rows);
        let state = match &self.backend.onboarding {
            Some(store) => store.get().await?,
            None => None,
        };
        let memory = self.backend.onboarding.is_none();
        match decide(Row::of(state.as_ref()), parsed.is_ok(), memory) {
            // `Fail`: `parsed` - ошибка, и `?` завершает процесс сообщением с ключом и причиной.
            Step::Ready | Step::Fail => Ok(Next::Ready(Box::new(parsed?), rows)),
            Step::Preconfigure => {
                let store = self.backend.onboarding()?;
                preconfigure(store.as_ref(), &SystemClock, &self.journal).await?;
                Ok(Next::Again)
            }
            Step::StartWizard => {
                let store = self.backend.onboarding()?;
                let source = self.source(&rows)?;
                start_wizard(store.as_ref(), &SystemClock, &self.journal, source).await?;
                Ok(Next::Again)
            }
            Step::Wizard => {
                let store = self.backend.onboarding()?;
                let source = self.source(&rows)?;
                refresh_token(
                    store.as_ref(),
                    &SystemClock,
                    &self.journal,
                    state.as_ref(),
                    source,
                )
                .await?;
                Ok(Next::Wizard)
            }
        }
    }

    fn source(&self, rows: &BTreeMap<String, Stored>) -> Result<Source, RunError> {
        let var = |name: &str| self.boot.var(name);
        let read = |path: &std::path::Path| self.boot.read(path);
        Ok(Source::resolve(&var, &read, stored_token_set(rows))?)
    }

    /// Публичный слушатель отвечает `503 not-configured` на всё, кроме маршрутов мастера (B4b);
    /// admin держит пробы. Фаза гасится опросом строки `onboarding` (одинаково на любой реплике)
    /// или сигналом.
    async fn serve_wizard(&self) -> Result<Stopped, RunError> {
        // Порты не зависят от результата разбора: они читаются только из env и файла.
        let var = |name: &str| self.boot.var(name);
        let ports = (http_port(&var)?, admin_port(&var)?);
        let store = Arc::clone(self.backend.onboarding()?);
        let stop = async move {
            let signal = Box::pin(shutdown_signal());
            let completed = Box::pin(until_completed(store, WIZARD_POLL));
            match future::select(signal, completed).await {
                Either::Left(_) => Stopped::Signal,
                Either::Right(_) => Stopped::Completed,
            }
        }
        .shared();
        let listeners = bind_both(ports).await?;
        tracing::info!(
            http_port = ports.0,
            admin_port = ports.1,
            "первый запуск не завершён: протоколы отвечают 503 not-configured"
        );
        run_servers(
            wizard_router(Router::new()),
            wizard_admin_router(),
            listeners,
            stop.clone(),
        )
        .await?;
        Ok(stop.await)
    }

    async fn serve_ready(
        self,
        config: Config,
        rows: &BTreeMap<String, Stored>,
    ) -> Result<(), RunError> {
        let (http_port, admin_port) = (config.http_port, config.admin_port);
        let names = |format, kind| config.repos.names(format, kind).to_vec();
        let repos = names(Format::Cargo, RepositoryKind::Hosted);
        let proxy_repos = names(Format::Cargo, RepositoryKind::Proxy);
        let npm_repos = names(Format::Npm, RepositoryKind::Hosted);
        let npm_proxy_repos = names(Format::Npm, RepositoryKind::Proxy);
        let (api, admin) = routers(self.backend.into_loaded(config, rows)).await?;
        let listeners = bind_both((http_port, admin_port)).await?;
        tracing::info!(
            http_port,
            admin_port,
            repos = ?repos,
            proxy_repos = ?proxy_repos,
            npm_repos = ?npm_repos,
            npm_proxy_repos = ?npm_proxy_repos,
            "сервер запущен"
        );
        run_servers(api, admin, listeners, shutdown_signal().shared()).await?;
        tracing::info!("сервер остановлен");
        Ok(())
    }
}

async fn bind_both(ports: (u16, u16)) -> Result<(TcpListener, TcpListener), RunError> {
    Ok((bind(ports.0).await?, bind(ports.1).await?))
}

async fn bind(port: u16) -> Result<TcpListener, RunError> {
    TcpListener::bind(("0.0.0.0", port))
        .await
        .map_err(|error| format!("не удалось занять порт {port}: {error}").into())
}

/// Оба сервера ждут одну и ту же остановку: слушатели освобождаются до выхода из функции,
/// поэтому следующая фаза занимает те же порты.
async fn run_servers(
    api: Router,
    admin: Router,
    (api_listener, admin_listener): (TcpListener, TcpListener),
    stop: impl Future + Clone + Send + 'static,
) -> Result<(), RunError> {
    let api_server = axum::serve(
        api_listener,
        // Адрес пира нужен слою контекста запроса (доверие к X-Forwarded-For).
        api.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(stop.clone().map(drop))
    .into_future();
    let admin_server = axum::serve(admin_listener, admin)
        .with_graceful_shutdown(stop.map(drop))
        .into_future();
    future::try_join(api_server, admin_server).await?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/phases.rs"]
mod tests;
