//! Общее для ответов всех форматов: распаковка итога proxy-движка и документы с условными запросами.

mod names;
mod respond;

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use axum_extra::headers::{ETag, IfNoneMatch};
use bytes::Bytes;
use crane_domain::registry::Sha256 as Digest;
use sha2::{Digest as _, Sha256};

pub use names::EngineUpstreamNames;
pub(crate) use respond::{Refusal, refuse, respond, with_facts};

use crate::error::ClientError;

/// Ответ с телом документа (индекс, packument): общее правило для hosted и proxy. `ETag` считается
/// по тем самым байтам, что уходят клиенту, поэтому условный запрос работает и для проксированного
/// документа. `vary_accept` просит формат, у которого вид тела зависит от `Accept`.
pub(crate) fn document_response(
    body: Bytes,
    if_none_match: Option<TypedHeader<IfNoneMatch>>,
    content_type: &'static str,
    vary_accept: bool,
) -> Result<Response, ClientError> {
    let etag = etag_of(&body)?;
    let unchanged =
        if_none_match.is_some_and(|TypedHeader(condition)| !condition.precondition_passes(&etag));
    let mut response = if unchanged {
        (StatusCode::NOT_MODIFIED, TypedHeader(etag)).into_response()
    } else {
        (
            TypedHeader(etag),
            [(header::CONTENT_TYPE, HeaderValue::from_static(content_type))],
            body,
        )
            .into_response()
    };
    if vary_accept {
        response
            .headers_mut()
            .insert(header::VARY, HeaderValue::from_static("Accept"));
    }
    Ok(response)
}

fn etag_of(body: &[u8]) -> Result<ETag, ClientError> {
    let digest = Digest::from(<[u8; 32]>::from(Sha256::digest(body)));
    format!("\"{digest}\"")
        .parse()
        .map_err(|error| ClientError::internal(&error))
}
