//! Диспетчер канала инвалидации: подписка, разбор сообщений, реакция на потерю связи.

use std::sync::Arc;
use std::time::Duration;

use fred::clients::SubscriberClient;
use fred::interfaces::{ClientLike, EventInterface, PubsubInterface};
use tokio::sync::broadcast::error::RecvError;
use tracing::warn;

use super::{AbortOnDrop, INVALIDATION_CHANNEL, Local};

/// Пауза между повторами, когда подключение или подписка не удались.
const RETRY_DELAY: Duration = Duration::from_secs(5);

/// Подключает клиента, повторяя попытки: Valkey может быть недоступен при старте.
/// В лог попадает только вид ошибки: текст может цитировать адрес.
pub(super) async fn connect<C: ClientLike>(
    client: &C,
) -> AbortOnDrop<Result<(), fred::error::Error>> {
    loop {
        match client.init().await {
            Ok(handle) => return AbortOnDrop(handle),
            Err(error) => {
                warn!(
                    kind = error.kind().to_str(),
                    "Valkey: подключение не удалось, повтор"
                );
                tokio::time::sleep(RETRY_DELAY).await;
            }
        }
    }
}

/// Пауза после завершения задачи соединения; в лог идёт только вид ошибки.
pub(super) async fn connection_lost(
    outcome: Result<Result<(), fred::error::Error>, tokio::task::JoinError>,
) {
    let kind = match outcome {
        Ok(Err(error)) => error.kind().to_str(),
        Ok(Ok(())) | Err(_) => "closed",
    };
    warn!(kind, "Valkey: соединение потеряно, переподключение");
    tokio::time::sleep(RETRY_DELAY).await;
}

/// Подписывается на `inv` и применяет сообщения к локальному состоянию, пока жив клиент.
///
/// После каждого (пере)подключения подписка ставится заново, а L1 сбрасывается: за время
/// без подписки инвалидации потеряны.
pub(super) async fn run(local: Arc<Local>, subscriber: SubscriberClient) {
    let mut messages = subscriber.message_rx();
    let mut reconnects = subscriber.reconnect_rx();
    let mut connection = connect(&subscriber).await;
    loop {
        while let Err(error) = subscriber.subscribe(INVALIDATION_CHANNEL).await {
            warn!(
                kind = error.kind().to_str(),
                "Valkey: подписка на inv не удалась, повтор"
            );
            tokio::time::sleep(RETRY_DELAY).await;
        }
        local.forget_all();
        loop {
            tokio::select! {
                message = messages.recv() => match message {
                    Ok(message) => match message.value.as_string() {
                        Some(name) => local.invalidate(&name),
                        None => local.forget_all(),
                    },
                    Err(RecvError::Lagged(_)) => local.forget_all(),
                    Err(RecvError::Closed) => return,
                },
                event = reconnects.recv() => match event {
                    Ok(_) | Err(RecvError::Lagged(_)) => break,
                    Err(RecvError::Closed) => return,
                },
                // Задача соединения завершилась: после повторного подключения подписка
                // ставится заново, а L1 сбрасывается (внешний цикл).
                outcome = &mut connection.0 => {
                    connection_lost(outcome).await;
                    connection = connect(&subscriber).await;
                    break;
                }
            }
        }
    }
}
