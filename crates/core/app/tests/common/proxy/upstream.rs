//! Скриптованный upstream: ответы по очереди, журнал вызовов, гейт на первом вызове.

use std::collections::VecDeque;
use std::io;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use bytes::Bytes;
use crane_domain::proxy::EgressRefusal;
use crane_ports::proxy::{ETag, Upstream, UpstreamError, UpstreamRequest, UpstreamResponse};
use crane_ports::registry::ByteStream;
use futures_channel::oneshot;
use futures_util::stream;

/// Один шаг скрипта upstream.
#[derive(Debug, Clone)]
pub(crate) enum Step {
    NotModified,
    /// 304 с обновлённым `Cache-Control: max-age`.
    NotModifiedWithMaxAge(Duration),
    Found(&'static [u8], Option<&'static str>),
    /// 200 с объявленным `Cache-Control: max-age`.
    FoundWithMaxAge(&'static [u8], Option<&'static str>, Duration),
    /// 200, тело приходит несколькими чанками.
    FoundChunks(&'static [&'static [u8]]),
    NotFound,
    Refused(EgressRefusal),
    Unavailable,
    /// 429/403: upstream ограничил запросы.
    Throttled,
    /// Тело оборвалось на чтении.
    BrokenBody,
}

/// Вызов upstream, как его увидел фейк.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) path: String,
    pub(crate) if_none_match: Option<String>,
}

#[derive(Debug, Default)]
struct UpstreamState {
    script: Mutex<VecDeque<Step>>,
    calls: Mutex<Vec<Call>>,
    gate: Mutex<Option<oneshot::Receiver<()>>>,
}

/// Открывает гейт: до этого первый `fetch` висит.
#[derive(Debug)]
pub(crate) struct Gate(oneshot::Sender<()>);

impl Gate {
    pub(crate) fn open(self) {
        let _ = self.0.send(());
    }
}

/// Upstream по сценарию: ответы по очереди, журнал вызовов, опциональный гейт на первом вызове.
#[derive(Debug, Clone)]
pub(crate) struct FakeUpstream(Arc<UpstreamState>);

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap()
}

impl FakeUpstream {
    pub(crate) fn new(script: impl IntoIterator<Item = Step>) -> Self {
        let state = UpstreamState::default();
        locked(&state.script).extend(script);
        Self(Arc::new(state))
    }

    pub(crate) fn gated(script: impl IntoIterator<Item = Step>) -> (Self, Gate) {
        let upstream = Self::new(script);
        let (sender, receiver) = oneshot::channel();
        *locked(&upstream.0.gate) = Some(receiver);
        (upstream, Gate(sender))
    }

    pub(crate) fn calls(&self) -> Vec<Call> {
        locked(&self.0.calls).clone()
    }

    pub(crate) fn call_count(&self) -> usize {
        locked(&self.0.calls).len()
    }
}

fn stream_of(chunks: Vec<Result<Bytes, io::Error>>) -> ByteStream {
    Box::pin(stream::iter(chunks))
}

impl Upstream for FakeUpstream {
    async fn fetch(&self, request: UpstreamRequest<'_>) -> Result<UpstreamResponse, UpstreamError> {
        locked(&self.0.calls).push(Call {
            path: request.path.as_str().to_owned(),
            if_none_match: request.if_none_match.map(|etag| etag.as_str().to_owned()),
        });
        let gate = locked(&self.0.gate).take();
        if let Some(gate) = gate {
            let _ = gate.await;
        }
        let step = locked(&self.0.script).pop_front();
        match step {
            Some(Step::NotModified) => Ok(UpstreamResponse::NotModified { max_age: None }),
            Some(Step::NotModifiedWithMaxAge(max_age)) => Ok(UpstreamResponse::NotModified {
                max_age: Some(max_age),
            }),
            Some(Step::Found(body, etag)) => Ok(UpstreamResponse::Found {
                body: stream_of(vec![Ok(Bytes::from_static(body))]),
                etag: etag.map(|etag| ETag::new(etag.to_owned())),
                max_age: None,
            }),
            Some(Step::FoundWithMaxAge(body, etag, max_age)) => Ok(UpstreamResponse::Found {
                body: stream_of(vec![Ok(Bytes::from_static(body))]),
                etag: etag.map(|etag| ETag::new(etag.to_owned())),
                max_age: Some(max_age),
            }),
            Some(Step::FoundChunks(chunks)) => Ok(UpstreamResponse::Found {
                body: stream_of(
                    chunks
                        .iter()
                        .map(|chunk| Ok(Bytes::from_static(chunk)))
                        .collect(),
                ),
                etag: None,
                max_age: None,
            }),
            Some(Step::NotFound) => Ok(UpstreamResponse::NotFound),
            Some(Step::Refused(refusal)) => Err(UpstreamError::Refused(refusal)),
            Some(Step::BrokenBody) => Ok(UpstreamResponse::Found {
                body: stream_of(vec![Err(io::Error::other("обрыв"))]),
                etag: None,
                max_age: None,
            }),
            Some(Step::Throttled) => Err(UpstreamError::Throttled),
            Some(Step::Unavailable) | None => {
                Err(UpstreamError::Unavailable("upstream лежит".into()))
            }
        }
    }
}
