use std::io;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_domain::proxy::{
    Freshness, MetadataPolicy, NegativePolicy, ProxyKey, UpstreamBase, UpstreamPath,
};
use crane_domain::registry::Checksum;
use crane_ports::proxy::{
    Cached, CachedState, Claim, ETag, FetchLeases, Lease, ProxyCache, Upstream, UpstreamError,
    UpstreamRequest, UpstreamResponse,
};
use crane_ports::registry::{BlobError, BlobStore, ByteStream, Digests};
use futures_util::{StreamExt, stream};

use super::flight::Outcome;
use super::{Inner, ProxyError, UpstreamFailure};

/// Что именно выбираем: от этого зависят условный запрос, срок хранения и обработка тела.
#[derive(Debug, Clone, Copy)]
pub(super) enum Target {
    Document {
        policy: MetadataPolicy,
    },
    Artifact {
        expected: Option<Checksum>,
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
            (
                CachedState::Document {
                    unchanged, max_age, ..
                },
                Self::Document { policy },
            ) => policy.assess(*unchanged, *max_age, entry.fetched_at, now),
            (CachedState::Blob(digests), Self::Artifact { expected, .. })
                if expected.is_none_or(|expected| digests.satisfies(&expected)) =>
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
    pub(super) fn check_digest(&self, digests: &Digests) -> Result<(), ProxyError> {
        match self {
            Self::Artifact {
                expected: Some(expected),
                ..
            } if !digests.satisfies(expected) => Err(ProxyError::DigestMismatch {
                expected: Box::new(*expected),
                actual: Box::new(digests.of_kind(expected)),
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

/// Предыдущий документ: что нужно, чтобы подтвердить его по 304 или сравнить с новым 200.
struct PrevDocument<'a> {
    body: &'a Bytes,
    etag: Option<&'a ETag>,
    unchanged: u32,
    max_age: Option<Duration>,
}

fn prev_document(job: &Job) -> Option<PrevDocument<'_>> {
    match job.prev.as_ref().map(|prev| &prev.state) {
        Some(CachedState::Document {
            body,
            etag,
            unchanged,
            max_age,
        }) => Some(PrevDocument {
            body,
            etag: etag.as_ref(),
            unchanged: *unchanged,
            max_age: *max_age,
        }),
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

/// Состояние записи для кэша и срок её хранения.
type Stored = (CachedState, Duration);

async fn fetch<U: Upstream, C: ProxyCache, L: FetchLeases, B: BlobStore>(
    inner: &Inner<U, C, L, B>,
    job: &Job,
) -> Outcome {
    let response = ask_upstream(&inner.upstream, job).await?;
    // Время берём после ответа: `fetched_at` отсчитывает свежесть от момента получения, а не запроса.
    let fetched_at = inner.clock.now();
    let (state, keep_for) = match (response, job.target) {
        (UpstreamResponse::NotModified { max_age }, Target::Document { policy }) => {
            confirm_document(job, &policy, max_age)?
        }
        (UpstreamResponse::NotModified { .. }, Target::Artifact { .. }) => {
            return Err(UpstreamFailure::Unavailable.into());
        }
        (
            UpstreamResponse::Found {
                body,
                etag,
                max_age,
            },
            Target::Document { policy },
        ) => {
            let body = collect(body, inner.limits.max_document_bytes).await?;
            replace_document(job, &policy, body, etag, max_age)
        }
        (UpstreamResponse::Found { body, .. }, Target::Artifact { .. }) => {
            store_artifact(&inner.blobs, job, body, inner.limits.max_artifact_bytes).await?
        }
        (UpstreamResponse::NotFound, target) => record_miss(job, &target),
    };
    let entry = Cached { fetched_at, state };
    // Кэш не источник истины: не записался — следующий запрос просто сходит в upstream.
    let _ = inner.cache.put(&job.key, entry.clone(), keep_for).await;
    Ok(entry)
}

/// Запрос к upstream; отказ egress и недоступность различаются, детали сбоя наружу не идут.
async fn ask_upstream<U: Upstream>(
    upstream: &U,
    job: &Job,
) -> Result<UpstreamResponse, UpstreamFailure> {
    // Условные запросы только для документов: артефакт неизменяем, его нечем «подтверждать».
    let validator = match job.target {
        Target::Document { .. } => prev_document(job).and_then(|prev| prev.etag),
        Target::Artifact { .. } => None,
    };
    upstream
        .fetch(UpstreamRequest {
            base: &job.base,
            path: &job.path,
            if_none_match: validator,
        })
        .await
        .map_err(|error| match error {
            UpstreamError::Refused(refusal) => UpstreamFailure::Refused(refusal),
            UpstreamError::Unavailable(_) => UpstreamFailure::Unavailable,
        })
}

/// 304: прежний документ подтверждён, счётчик «не изменился» растёт, срок обновляется.
fn confirm_document(
    job: &Job,
    policy: &MetadataPolicy,
    max_age: Option<Duration>,
) -> Result<Stored, ProxyError> {
    // 304 без прежнего документа — сбой протокола upstream: отдавать нечего.
    let prev = prev_document(job).ok_or(UpstreamFailure::Unavailable)?;
    let unchanged = prev.unchanged.saturating_add(1);
    // Свежий срок из 304 важнее (RFC 9111 §4.3.4); без него действует последний объявленный.
    let max_age = max_age.or(prev.max_age);
    let state = CachedState::Document {
        body: prev.body.clone(),
        etag: prev.etag.cloned(),
        unchanged,
        max_age,
    };
    Ok((state, policy.keep_for(unchanged, max_age)))
}

/// 200 с документом: новое тело, счётчик «не изменился» считается сравнением с прежним.
fn replace_document(
    job: &Job,
    policy: &MetadataPolicy,
    body: Bytes,
    etag: Option<ETag>,
    max_age: Option<Duration>,
) -> Stored {
    // 304 и «200 с тем же телом» одинаково значат «не изменилось».
    let unchanged = prev_document(job)
        .filter(|prev| *prev.body == body)
        .map_or(0, |prev| prev.unchanged.saturating_add(1));
    let state = CachedState::Document {
        body,
        etag,
        unchanged,
        max_age,
    };
    (state, policy.keep_for(unchanged, max_age))
}

/// 200 с артефактом: staging с потолком размера, проверка дайджеста, коммит в CAS.
async fn store_artifact<B: BlobStore>(
    blobs: &B,
    job: &Job,
    body: ByteStream,
    max_bytes: u64,
) -> Result<Stored, ProxyError> {
    // Потолок ставим до staging: сбежавший поток не должен забить диск временной зоны.
    let bounded = bounded(body, max_bytes);
    let staged = blobs.stage(bounded).await.map_err(|error| match error {
        BlobError::Body(error) => body_failure(&error),
        other => ProxyError::backend(other),
    })?;
    // Несовпавший staged-объект не коммитим: чужие байты не должны попасть в CAS.
    job.target.check_digest(staged.digests())?;
    let digests = blobs.commit(staged).await.map_err(ProxyError::backend)?;
    // Содержимое неизменяемо: запись без срока, вытеснение — право бэкенда.
    Ok((CachedState::Blob(digests), Duration::MAX))
}

/// 404: отрицательная запись, счётчик промахов растёт от прежней отрицательной записи.
fn record_miss(job: &Job, target: &Target) -> Stored {
    let misses = match job.prev.as_ref().map(|prev| &prev.state) {
        Some(CachedState::Missing { misses }) => misses.saturating_add(1),
        _ => 1,
    };
    (
        CachedState::Missing { misses },
        target.negative().ttl_after(misses),
    )
}

/// Маркер переполнения в ошибке потока: по нему `body_failure` отличает превышение от обрыва.
#[derive(Debug, thiserror::Error)]
#[error("тело upstream больше {limit} байт")]
struct Overflow {
    limit: u64,
}

/// Поток с потолком: чанк, переводящий сумму за `limit`, не отдаётся, вместо него ошибка `Overflow`.
/// Сверх лимита дальше по конвейеру не уходит ни байта, после ошибки поток завершается.
fn bounded(body: ByteStream, limit: u64) -> ByteStream {
    Box::pin(stream::unfold(
        Some((body, 0_u64)),
        move |state| async move {
            let (mut body, seen) = state?;
            match body.next().await? {
                Err(error) => Some((Err(error), None)),
                Ok(chunk) => {
                    let total = seen.saturating_add(u64::try_from(chunk.len()).unwrap_or(u64::MAX));
                    if total > limit {
                        Some((Err(io::Error::other(Overflow { limit })), None))
                    } else {
                        Some((Ok(chunk), Some((body, total))))
                    }
                }
            }
        },
    ))
}

/// Единственное место, где сбой тела потока становится `ProxyError`.
fn body_failure(error: &io::Error) -> ProxyError {
    match error
        .get_ref()
        .and_then(|inner| inner.downcast_ref::<Overflow>())
    {
        Some(overflow) => ProxyError::TooLarge {
            limit: overflow.limit,
        },
        None => UpstreamFailure::Unavailable.into(),
    }
}

/// Собирает тело документа в память с потолком: документ целиком нужен для сравнения и отдачи.
async fn collect(body: ByteStream, limit: u64) -> Result<Bytes, ProxyError> {
    let mut body = bounded(body, limit);
    let mut buffer = Vec::new();
    while let Some(chunk) = body.next().await {
        // Потолок срабатывает до копирования: гигантский ответ не раздувает память.
        buffer.extend_from_slice(&chunk.map_err(|error| body_failure(&error))?);
    }
    Ok(Bytes::from(buffer))
}
