use std::sync::Arc;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_domain::proxy::{
    Freshness, MetadataPolicy, NegativePolicy, ProxyKey, UpstreamBase, UpstreamPath,
};
use crane_domain::registry::Sha256;
use crane_ports::proxy::{
    Cached, CachedState, Claim, ETag, FetchLeases, Lease, ProxyCache, Upstream, UpstreamError,
    UpstreamRequest, UpstreamResponse,
};
use crane_ports::registry::{BlobError, BlobStore, ByteStream};
use futures_util::StreamExt;

use super::flight::Outcome;
use super::{Inner, ProxyError, UpstreamFailure};

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

    /// Единственное место, где решается, годится ли лежащая запись этому запросу и насколько она свежа.
    /// Запись другого вида (документ под ключом артефакта и наоборот) или blob с чужим дайджестом — `Expired`.
    pub(super) fn freshness(&self, entry: &Cached, now: SystemTime) -> Freshness {
        match (&entry.state, self) {
            (CachedState::Document { unchanged, .. }, Self::Document { policy }) => {
                policy.assess(*unchanged, entry.fetched_at, now)
            }
            (CachedState::Blob(digests), Self::Artifact { expected, .. })
                if expected.is_none_or(|expected| expected == digests.sha256) =>
            {
                Freshness::Fresh
            }
            (CachedState::Missing { misses }, target)
                if !target.negative().is_expired(*misses, entry.fetched_at, now) =>
            {
                Freshness::Fresh
            }
            _ => Freshness::Expired,
        }
    }

    pub(super) fn accepts(&self, entry: &Cached, now: SystemTime) -> bool {
        self.freshness(entry, now) != Freshness::Expired
    }

    /// Дайджест полученных байт против ожидаемого. Полёт общий, а ожидание у каждого вызывающего своё,
    /// поэтому проверка нужна и лидеру до коммита, и присоединившемуся после полёта.
    pub(super) fn check_digest(&self, actual: Sha256) -> Result<(), ProxyError> {
        match self {
            Self::Artifact {
                expected: Some(expected),
                ..
            } if *expected != actual => Err(ProxyError::DigestMismatch {
                expected: *expected,
                actual,
            }),
            _ => Ok(()),
        }
    }
}

/// Задание на выборку: всё, что нужно полёту, владеемое, потому что полёт переживает вызвавший его запрос.
#[derive(Debug, Clone)]
pub(super) struct Job {
    pub(super) key: ProxyKey,
    pub(super) base: UpstreamBase,
    pub(super) path: UpstreamPath,
    pub(super) target: Target,
    pub(super) prev: Option<Cached>,
}

/// Тело, `ETag` и счётчик необновлений предыдущего документа.
fn prev_document(job: &Job) -> Option<(&Bytes, Option<&ETag>, u32)> {
    match job.prev.as_ref().map(|prev| &prev.state) {
        Some(CachedState::Document {
            body,
            etag,
            unchanged,
        }) => Some((body, etag.as_ref(), *unchanged)),
        _ => None,
    }
}

/// Владение lease. Лидера могут бросить посреди выборки (клиент оборвал запрос), а `Drop` не умеет `await`:
/// снятие начинается синхронно при вызове `release` (контракт порта), поэтому future можно не опрашивать.
struct Held<'a, L: FetchLeases> {
    leases: &'a L,
    lease: Option<Lease>,
}

impl<L: FetchLeases> Drop for Held<'_, L> {
    fn drop(&mut self) {
        if let Some(lease) = self.lease.take() {
            drop(self.leases.release(lease));
        }
    }
}

/// Выборка лидером: межрепликовый lease, затем сама выборка.
///
/// Недоступный координатор не ошибка запроса (10.2): реплики тогда ходят в upstream независимо.
pub(super) async fn lead<U: Upstream, C: ProxyCache, L: FetchLeases, B: BlobStore>(
    inner: Arc<Inner<U, C, L, B>>,
    job: Job,
) -> Outcome {
    match inner
        .leases
        .claim(&job.key, inner.limits.lease, inner.limits.patience)
        .await
    {
        Ok(Claim::Lead(lease)) => {
            let held = Held {
                leases: &inner.leases,
                lease: Some(lease),
            };
            let outcome = fetch(&inner, &job).await;
            // Снятие — в Drop: единый путь для успеха, ошибки и отмены. Подтверждение не ждём:
            // по контракту порта снятие начинается при вызове, а ошибка release не влияет на результат.
            drop(held);
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
    job.target
        .accepts(&entry, inner.clock.now())
        .then_some(entry)
}

async fn fetch<U: Upstream, C: ProxyCache, L: FetchLeases, B: BlobStore>(
    inner: &Inner<U, C, L, B>,
    job: &Job,
) -> Outcome {
    // Условные запросы только для документов: артефакт неизменяем, его нечем «подтверждать».
    let validator = match job.target {
        Target::Document { .. } => prev_document(job).and_then(|(_, etag, _)| etag),
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
            UpstreamError::Refused(refusal) => UpstreamFailure::Refused(refusal),
            UpstreamError::Unavailable(_) => UpstreamFailure::Unavailable,
        })?;
    // Время берём после ответа: `fetched_at` отсчитывает свежесть от момента получения, а не запроса.
    let fetched_at = inner.clock.now();
    let (state, keep_for) = match (response, job.target) {
        (UpstreamResponse::NotModified, Target::Document { policy }) => {
            // 304 без прежнего документа — сбой протокола upstream: отдавать нечего.
            let (body, etag, unchanged) = prev_document(job).ok_or(UpstreamFailure::Unavailable)?;
            let unchanged = unchanged.saturating_add(1);
            let state = CachedState::Document {
                body: body.clone(),
                etag: etag.cloned(),
                unchanged,
            };
            (state, policy.keep_for(unchanged))
        }
        (UpstreamResponse::NotModified, Target::Artifact { .. }) => {
            return Err(UpstreamFailure::Unavailable.into());
        }
        (UpstreamResponse::Found { body, etag }, Target::Document { policy }) => {
            let body = collect(body, inner.limits.max_document_bytes).await?;
            // 304 и «200 с тем же телом» одинаково значат «не изменилось».
            let unchanged = prev_document(job)
                .filter(|(prev, _, _)| **prev == body)
                .map_or(0, |(_, _, unchanged)| unchanged.saturating_add(1));
            let state = CachedState::Document {
                body,
                etag,
                unchanged,
            };
            (state, policy.keep_for(unchanged))
        }
        (UpstreamResponse::Found { body, .. }, Target::Artifact { .. }) => {
            let staged = inner.blobs.stage(body).await.map_err(|error| match error {
                BlobError::Body(_) => ProxyError::from(UpstreamFailure::Unavailable),
                other => ProxyError::backend(other),
            })?;
            // Несовпавший staged-объект не коммитим: чужие байты не должны попасть в CAS.
            job.target.check_digest(staged.digests().sha256)?;
            let digests = inner
                .blobs
                .commit(staged)
                .await
                .map_err(ProxyError::backend)?;
            // Содержимое неизменяемо: запись без срока, вытеснение — право бэкенда.
            (CachedState::Blob(digests), Duration::MAX)
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
async fn collect(mut body: ByteStream, limit: u64) -> Result<Bytes, ProxyError> {
    let mut buffer = Vec::new();
    while let Some(chunk) = body.next().await {
        let chunk = chunk.map_err(|_| UpstreamFailure::Unavailable)?;
        let total = u64::try_from(buffer.len())
            .unwrap_or(u64::MAX)
            .saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
        // Проверка до копирования: гигантский ответ не должен раздувать память.
        if total > limit {
            return Err(ProxyError::TooLarge { limit });
        }
        buffer.extend_from_slice(&chunk);
    }
    Ok(Bytes::from(buffer))
}
