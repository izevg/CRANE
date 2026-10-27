//! Единая распаковка итога движка: заголовки кэша, факты для журнала, фоновая ревалидация.

use axum::http::{HeaderValue, header};
use axum::response::{IntoResponse, Response};
use crane_app::proxy::{CacheResult, ProxyError, Revalidation, Served};
use tracing::{Instrument, Span, debug, warn};

use crate::context::CacheFacts;
use crate::error::{CargoError, chain};

// Warning отменён RFC 9111 и оставлен по требованию; основной сигнал — `Age` и поле `cache` журнала.
const STALE: HeaderValue = HeaderValue::from_static("110 - \"Response is Stale\"");
const REVALIDATION_FAILED: HeaderValue = HeaderValue::from_static("111 - \"Revalidation Failed\"");

/// Превращает итог движка в ответ. Здесь же: запуск фоновой ревалидации, `Age`, `Warning`,
/// `CacheFacts` для успеха и для ошибки. `ok` строит тело из полученного значения.
pub(super) fn respond<T>(
    result: Result<Served<T>, ProxyError>,
    ok: impl FnOnce(T) -> Result<Response, CargoError>,
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
        Err(error) => failure(error),
    }
}

/// Ответ на ошибку движка: тело для клиента без деталей, факты для журнала.
pub(super) fn failure(error: ProxyError) -> Response {
    let facts = CacheFacts::failed(&error);
    with_facts(CargoError::from(error).into_response(), facts)
}

pub(super) fn with_facts(mut response: Response, facts: CacheFacts) -> Response {
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
