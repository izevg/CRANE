//! Единая распаковка итога движка: заголовки кэша, факты для журнала, фоновая ревалидация.

use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use crane_app::proxy::{CacheResult, ProxyError, Revalidation, Served};
use crane_app::registry::RegistryError;
use tracing::{Instrument, Span, debug, warn};

use crate::context::CacheFacts;
use crate::error::{ClientError, chain};

// Warning отменён RFC 9111 и оставлен по требованию; основной сигнал — `Age` и поле `cache` журнала.
const STALE: HeaderValue = HeaderValue::from_static("110 - \"Response is Stale\"");
const REVALIDATION_FAILED: HeaderValue = HeaderValue::from_static("111 - \"Revalidation Failed\"");

/// Почему документ не получен: защита внутренних имён отказала до движка или движок не справился.
/// Общая для всех форматов, потому что точка входа в движок у каждого формата одна и защита стоит перед ней.
pub(crate) enum Refusal {
    Registry(RegistryError),
    Proxy(ProxyError),
}

impl From<ProxyError> for Refusal {
    fn from(error: ProxyError) -> Self {
        Self::Proxy(error)
    }
}

/// Превращает итог движка в ответ. Здесь же: запуск фоновой ревалидации, `Age`, `Warning`,
/// `CacheFacts` для успеха и для ошибки. `ok` строит тело из полученного значения.
pub(crate) fn respond<T, E: Into<Refusal>>(
    result: Result<Served<T>, E>,
    ok: impl FnOnce(T) -> Result<Response, ClientError>,
) -> Response {
    match result {
        Ok(served) => {
            let Served {
                body,
                result,
                age,
                revalidation,
            } = served;
            if let Some(revalidation) = revalidation {
                spawn_revalidation(revalidation);
            }
            let mut response = ok(body).unwrap_or_else(IntoResponse::into_response);
            let headers = response.headers_mut();
            headers.insert(header::AGE, HeaderValue::from(age.as_secs()));
            match result {
                CacheResult::Stale => {
                    headers.insert(header::WARNING, STALE);
                }
                CacheResult::StaleOnError(_) => {
                    headers.insert(header::WARNING, REVALIDATION_FAILED);
                }
                CacheResult::Hit | CacheResult::Miss => {}
            }
            with_facts(response, CacheFacts::served(&result))
        }
        Err(refusal) => refuse(refusal.into()),
    }
}

/// Отказ защиты имён даёт ответ без фактов кэша: до кэша и upstream дело не дошло.
pub(crate) fn refuse(refusal: Refusal) -> Response {
    match refusal {
        Refusal::Registry(error) => ClientError::from(error).into_response(),
        Refusal::Proxy(error) => failure(error),
    }
}

/// Ответ на ошибку движка: тело для клиента без деталей, факты для журнала.
pub(crate) fn failure(error: ProxyError) -> Response {
    let facts = CacheFacts::failed(&error);
    with_facts(ClientError::from(error).into_response(), facts)
}

pub(crate) fn with_facts(mut response: Response, facts: CacheFacts) -> Response {
    response.extensions_mut().insert(facts);
    response
}

/// У фоновой ревалидации своей строки журнала нет, поэтому итог пишется отдельно;
/// span запроса переносит `request_id` в запись.
fn spawn_revalidation(revalidation: Revalidation) {
    tokio::spawn(
        async move {
            match revalidation.await {
                Ok(()) => debug!("фоновое обновление proxy выполнено"),
                Err(error) => warn!(error = %chain(&error), "фоновое обновление proxy не удалось"),
            }
        }
        .instrument(Span::current()),
    );
}
