use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use crane_domain::proxy::{
    Freshness, MetadataPolicy, NegativePolicy, ProxyKey, UpstreamBase, UpstreamPath,
};
use crane_domain::registry::Sha256;
use crane_ports::proxy::{
    Cached, CachedBody, CachedState, Claim, ETag, FetchLeases, ProxyCache, Upstream, UpstreamError,
    UpstreamRequest, UpstreamResponse,
};
use crane_ports::registry::{BlobError, BlobStore, ByteStream};
use futures_util::StreamExt;

use super::Inner;
use super::flight::Failure;

/// Что именно выбираем: от этого зависят условный запрос, срок хранения и обработка тела.
#[derive(Debug, Clone, Copy)]
pub(super) enum Target {
    Document {
        policy: MetadataPolicy,
    },
    Artifact {
        expected: Option<Sha256>,
        negative: NegativePolicy,
    },
}

impl Target {
    const fn negative(&self) -> &NegativePolicy {
        match self {
            Self::Document { policy } => &policy.negative,
            Self::Artifact { negative, .. } => negative,
        }
    }
}

/// Задание на выборку: всё, что нужно полёту, владеемое, потому что полёт переживает вызвавший его запрос.
#[derive(Debug)]
pub(super) struct Job {
    pub(super) key: ProxyKey,
    pub(super) base: UpstreamBase,
    pub(super) path: UpstreamPath,
    pub(super) target: Target,
    pub(super) prev: Option<Cached>,
}

/// Тело, `ETag` и счётчик необновлений предыдущей записи-документа.
struct Document<'a> {
    body: &'a Bytes,
    etag: Option<&'a ETag>,
    unchanged: u32,
}

fn document(prev: Option<&Cached>) -> Option<Document<'_>> {
    match &prev?.state {
        CachedState::Found {
            body: CachedBody::Document(body),
            etag,
            unchanged,
        } => Some(Document {
            body,
            etag: etag.as_ref(),
            unchanged: *unchanged,
        }),
        _ => None,
    }
}

/// Выборка лидером: межрепликовый lease, затем сама выборка.
///
/// Недоступный координатор не ошибка запроса (10.2): реплики тогда ходят в upstream независимо.
pub(super) async fn lead<U: Upstream, C: ProxyCache, L: FetchLeases, B: BlobStore>(
    inner: Arc<Inner<U, C, L, B>>,
    job: Job,
) -> Result<Cached, Failure> {
    match inner
        .leases
        .claim(&job.key, inner.limits.lease, inner.limits.patience)
        .await
    {
        Ok(Claim::Lead(lease)) => {
            let outcome = fetch(&inner, &job).await;
            // Ошибка release не влияет на результат: lease истечёт сам.
            let _ = inner.leases.release(lease).await;
            outcome
        }
        Ok(Claim::Settled) => match settled(&inner, &job).await {
            Some(entry) => Ok(entry),
            None => fetch(&inner, &job).await,
        },
        Err(_) => fetch(&inner, &job).await,
    }
}

/// Запись, которую успел положить другой лидер, если она ещё годится для этого запроса.
async fn settled<U: Upstream, C: ProxyCache, L: FetchLeases, B: BlobStore>(
    inner: &Inner<U, C, L, B>,
    job: &Job,
) -> Option<Cached> {
    let entry = inner.cache.get(&job.key).await.ok().flatten()?;
    let now = inner.clock.now();
    let usable = match (&entry.state, &job.target) {
        (
            CachedState::Found {
                body: CachedBody::Document(_),
                unchanged,
                ..
            },
            Target::Document { policy },
        ) => policy.assess(*unchanged, entry.fetched_at, now) != Freshness::Expired,
        (
            CachedState::Found {
                body: CachedBody::Blob(digests),
                ..
            },
            Target::Artifact { expected, .. },
        ) => expected.is_none_or(|expected| expected == digests.sha256),
        (CachedState::Missing { misses }, target) => {
            !target.negative().is_expired(*misses, entry.fetched_at, now)
        }
        _ => false,
    };
    usable.then_some(entry)
}

async fn fetch<U: Upstream, C: ProxyCache, L: FetchLeases, B: BlobStore>(
    inner: &Inner<U, C, L, B>,
    job: &Job,
) -> Result<Cached, Failure> {
    // Условные запросы только для документов: артефакт неизменяем, его нечем «подтверждать».
    let validator = match job.target {
        Target::Document { .. } => document(job.prev.as_ref()).and_then(|prev| prev.etag),
        Target::Artifact { .. } => None,
    };
    let response = inner
        .upstream
        .fetch(UpstreamRequest {
            base: &job.base,
            path: &job.path,
            if_none_match: validator,
        })
        .await
        .map_err(|error| match error {
            UpstreamError::Refused(refusal) => Failure::Refused(refusal),
            UpstreamError::Unavailable(_) => Failure::Unavailable,
        })?;
    // Время берём после ответа: `fetched_at` отсчитывает свежесть от момента получения, а не запроса.
    let fetched_at = inner.clock.now();
    let (state, keep_for) = match (response, job.target) {
        (UpstreamResponse::NotModified, Target::Document { policy }) => {
            // 304 без прежнего документа — сбой протокола upstream: отдавать нечего.
            let prev = document(job.prev.as_ref()).ok_or(Failure::Unavailable)?;
            let unchanged = prev.unchanged.saturating_add(1);
            let state = CachedState::Found {
                body: CachedBody::Document(prev.body.clone()),
                etag: prev.etag.cloned(),
                unchanged,
            };
            (state, policy.keep_for(unchanged))
        }
        (UpstreamResponse::NotModified, Target::Artifact { .. }) => {
            return Err(Failure::Unavailable);
        }
        (UpstreamResponse::Found { body, etag }, Target::Document { policy }) => {
            let body = collect(body, inner.limits.max_document_bytes).await?;
            // 304 и «200 с тем же телом» одинаково значат «не изменилось».
            let unchanged = document(job.prev.as_ref())
                .filter(|prev| *prev.body == body)
                .map_or(0, |prev| prev.unchanged.saturating_add(1));
            let state = CachedState::Found {
                body: CachedBody::Document(body),
                etag,
                unchanged,
            };
            (state, policy.keep_for(unchanged))
        }
        (UpstreamResponse::Found { body, etag }, Target::Artifact { expected, .. }) => {
            let staged = inner.blobs.stage(body).await.map_err(|error| match error {
                BlobError::Body(_) => Failure::Unavailable,
                other => Failure::backend(other),
            })?;
            let actual = staged.digests().sha256;
            // Несовпавший staged-объект не коммитим: чужие байты не должны попасть в CAS.
            if let Some(expected) = expected
                && expected != actual
            {
                return Err(Failure::DigestMismatch { expected, actual });
            }
            let digests = inner.blobs.commit(staged).await.map_err(Failure::backend)?;
            let state = CachedState::Found {
                body: CachedBody::Blob(digests),
                etag,
                unchanged: 0,
            };
            // Содержимое неизменяемо: запись без срока, вытеснение — право бэкенда.
            (state, Duration::MAX)
        }
        (UpstreamResponse::NotFound, target) => {
            let misses = match job.prev.as_ref().map(|prev| &prev.state) {
                Some(CachedState::Missing { misses }) => misses.saturating_add(1),
                _ => 1,
            };
            (
                CachedState::Missing { misses },
                target.negative().ttl_after(misses),
            )
        }
    };
    let entry = Cached { fetched_at, state };
    // Кэш не источник истины: не записался — следующий запрос просто сходит в upstream.
    let _ = inner.cache.put(&job.key, entry.clone(), keep_for).await;
    Ok(entry)
}

/// Собирает тело документа в память с потолком: документ целиком нужен для сравнения и отдачи.
async fn collect(mut body: ByteStream, limit: u64) -> Result<Bytes, Failure> {
    let mut buffer = Vec::new();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|_| Failure::Unavailable)?;
        let total = u64::try_from(buffer.len())
            .unwrap_or(u64::MAX)
            .saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
        // Проверка до копирования: гигантский ответ не должен раздувать память.
        if total > limit {
            return Err(Failure::TooLarge { limit });
        }
        buffer.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(buffer))
}
