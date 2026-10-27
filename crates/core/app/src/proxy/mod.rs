//! Proxy-движок: fetch-through кэш metadata и неизменяемых артефактов, не знающий форматов.
//!
//! Алгоритм — 10.2, зоны свежести и сроки — 11.1. Формат передаёт путь и политику, получает байты
//! и факты для журнала доступа; ссылки в документах переписывает сам формат.

mod flight;
mod lead;
mod served;

use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_domain::proxy::{Freshness, MetadataPolicy, NegativePolicy, Origin, UpstreamPath, age};
use crane_domain::registry::Checksum;
use crane_ports::proxy::{Cached, CachedState, FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::{BlobError, BlobStore};
use crane_ports::system::Clock;
use futures_util::FutureExt;

use self::flight::{Flight, Flights, Invariant};
use self::lead::{Job, Target, lead};
pub use self::served::{CacheResult, Fetched, Revalidation, Served, UpstreamFailure};

/// Пределы движка. Умолчания — 10.2 (lease 30 с, ожидание 25 с) плюс потолки размера документа (память) и архива (диск staging).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProxyLimits {
    pub max_document_bytes: u64,
    pub max_artifact_bytes: u64,
    pub lease: Duration,
    pub patience: Duration,
}

impl ProxyLimits {
    pub const DEFAULT: Self = Self {
        // 64 МиБ.
        max_document_bytes: 67_108_864,
        // 2 ГиБ: предохранитель от сбежавшего потока, а не политика формата;
        // запас над колёсами PyPI класса torch.
        max_artifact_bytes: 2_147_483_648,
        lease: Duration::from_secs(30),
        patience: Duration::from_secs(25),
    };
}

/// `Clone`, потому что итог полёта общий для всех присоединившихся.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProxyError {
    #[error("нет у upstream")]
    NotFound,
    #[error("upstream не ответил")]
    Upstream(#[from] UpstreamFailure),
    #[error("контрольная сумма не совпала: ожидалась {expected}, получена {actual}")]
    // Checksum с sha512 велик: без Box вариант раздувает каждый `Result<_, ProxyError>`.
    DigestMismatch {
        expected: Box<Checksum>,
        actual: Box<Checksum>,
    },
    #[error("ответ upstream больше {limit} байт")]
    TooLarge { limit: u64 },
    #[error("ошибка бэкенда")]
    Backend(#[source] Arc<dyn std::error::Error + Send + Sync>),
}

impl ProxyError {
    fn backend(error: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Backend(Arc::new(error))
    }

    fn invariant(what: &'static str) -> Self {
        Self::backend(Invariant(what))
    }
}

struct Inner<U, C, L, B> {
    upstream: U,
    cache: C,
    leases: L,
    blobs: B,
    clock: Arc<dyn Clock>,
    limits: ProxyLimits,
    flights: Flights,
}

/// Proxy-движок. `Clone` дёшев: фоновая ревалидация владеет своей копией.
pub struct ProxyEngine<U, C, L, B>(Arc<Inner<U, C, L, B>>);

// Вручную: derive потребовал бы `Clone` от портов, а делить нужно только `Arc`.
impl<U, C, L, B> Clone for ProxyEngine<U, C, L, B> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<U, C, L, B> fmt::Debug for ProxyEngine<U, C, L, B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Порты и `dyn`-зависимости не Debug.
        f.debug_struct("ProxyEngine").finish_non_exhaustive()
    }
}

impl<U, C, L, B> ProxyEngine<U, C, L, B>
where
    U: Upstream + 'static,
    C: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
{
    #[must_use]
    pub fn new(
        upstream: U,
        cache: C,
        leases: L,
        blobs: B,
        clock: Arc<dyn Clock>,
        limits: ProxyLimits,
    ) -> Self {
        Self(Arc::new(Inner {
            upstream,
            cache,
            leases,
            blobs,
            clock,
            limits,
            flights: Flights::default(),
        }))
    }

    /// Документ upstream (индекс, packument): свежий из кэша, просроченный с фоновым обновлением
    /// или полученный блокирующей выборкой.
    pub async fn metadata(
        &self,
        origin: &Origin,
        path: &UpstreamPath,
        policy: &MetadataPolicy,
    ) -> Result<Served<Bytes>, ProxyError> {
        let now = self.0.clock.now();
        let key = origin.key(path);
        // Ошибка кэша — промах: кэш не источник истины.
        let cached = self.0.cache.get(&key).await.ok().flatten();
        let job = Job {
            key,
            base: origin.base.clone(),
            path: path.clone(),
            target: Target::Document { policy: *policy },
            prev: cached,
        };
        let zone = job
            .prev
            .as_ref()
            .map_or(Freshness::Expired, |entry| job.target.freshness(entry, now));
        if let Some(served) = self.serve_cached(&job, zone, now) {
            return served;
        }
        let stale = job.prev.clone();
        let outcome = self.fly(job).await;
        self.document_served(outcome, stale.as_ref(), policy)
    }

    /// Неизменяемый артефакт через CAS. Stale-if-error для артефактов нет (11.1): хит либо выборка.
    pub async fn artifact(
        &self,
        origin: &Origin,
        path: &UpstreamPath,
        expected: Option<Checksum>,
        negative: &NegativePolicy,
    ) -> Result<Served<Fetched>, ProxyError> {
        let now = self.0.clock.now();
        let key = origin.key(path);
        let cached = self.0.cache.get(&key).await.ok().flatten();
        let target = Target::Artifact {
            expected,
            negative: *negative,
        };
        let job = Job {
            key,
            base: origin.base.clone(),
            path: path.clone(),
            target,
            prev: cached,
        };
        if let Some(served) = self.cached_artifact(&job, now).await? {
            return Ok(served);
        }
        match self.fly(job).await? {
            Cached {
                fetched_at,
                state: CachedState::Blob(digests),
            } => {
                target.check_digest(&digests)?;
                let body = self
                    .0
                    .blobs
                    .open(&digests.sha256)
                    .await
                    .map_err(ProxyError::backend)?;
                Ok(Served {
                    body: Fetched { digests, body },
                    result: CacheResult::Miss,
                    age: age(fetched_at, self.0.clock.now()),
                    revalidation: None,
                })
            }
            Cached {
                state: CachedState::Missing { .. },
                ..
            } => Err(ProxyError::NotFound),
            Cached {
                state: CachedState::Document { .. },
                ..
            } => Err(ProxyError::invariant("под ключом артефакта лежит документ")),
        }
    }

    /// Ответ из лежащей записи, если она годится. `None` — идти в upstream блокирующей выборкой.
    fn serve_cached(
        &self,
        job: &Job,
        zone: Freshness,
        now: SystemTime,
    ) -> Option<Result<Served<Bytes>, ProxyError>> {
        let (result, refresh) = match zone {
            Freshness::Fresh => (CacheResult::Hit, false),
            // Последние 10% TTL клиенту ещё «свежо», обновление уже идёт.
            Freshness::RefreshAhead => (CacheResult::Hit, true),
            Freshness::Stale => (CacheResult::Stale, true),
            Freshness::Expired => return None,
        };
        let entry = job.prev.as_ref()?;
        match &entry.state {
            CachedState::Missing { .. } => Some(Err(ProxyError::NotFound)),
            CachedState::Document { body, .. } => Some(Ok(Served {
                body: body.clone(),
                result,
                age: age(entry.fetched_at, now),
                revalidation: refresh.then(|| self.revalidation(job.clone())),
            })),
            CachedState::Blob(_) => None,
        }
    }

    /// Хит артефакта из CAS; `NotFound` для лежащей отрицательной записи.
    async fn cached_artifact(
        &self,
        job: &Job,
        now: SystemTime,
    ) -> Result<Option<Served<Fetched>>, ProxyError> {
        let Some(entry) = job
            .prev
            .as_ref()
            .filter(|entry| job.target.freshness(entry, now) == Freshness::Fresh)
        else {
            return Ok(None);
        };
        match &entry.state {
            CachedState::Missing { .. } => Err(ProxyError::NotFound),
            CachedState::Blob(digests) => match self.0.blobs.open(&digests.sha256).await {
                Ok(body) => Ok(Some(Served {
                    body: Fetched {
                        digests: *digests,
                        body,
                    },
                    result: CacheResult::Hit,
                    age: age(entry.fetched_at, now),
                    revalidation: None,
                })),
                // Класс proxy-cache разрешено терять (09): запись без blob — промах, качаем заново.
                Err(BlobError::NotFound(_)) => Ok(None),
                Err(error) => Err(ProxyError::backend(error)),
            },
            CachedState::Document { .. } => Ok(None),
        }
    }

    /// Итог блокирующей выборки документа; при сбое upstream просроченная запись ещё может выручить.
    fn document_served(
        &self,
        outcome: Result<Cached, ProxyError>,
        stale: Option<&Cached>,
        policy: &MetadataPolicy,
    ) -> Result<Served<Bytes>, ProxyError> {
        let now = self.0.clock.now();
        match outcome {
            Ok(Cached {
                fetched_at,
                state: CachedState::Document { body, .. },
            }) => Ok(Served {
                body,
                result: CacheResult::Miss,
                age: age(fetched_at, now),
                revalidation: None,
            }),
            Ok(Cached {
                state: CachedState::Missing { .. },
                ..
            }) => Err(ProxyError::NotFound),
            Ok(Cached {
                state: CachedState::Blob(_),
                ..
            }) => Err(ProxyError::invariant("под ключом документа лежит артефакт")),
            Err(ProxyError::Upstream(reason)) => match stale {
                Some(Cached {
                    fetched_at,
                    state:
                        CachedState::Document {
                            body,
                            unchanged,
                            max_age,
                            ..
                        },
                }) if policy.usable_on_error(*unchanged, *max_age, *fetched_at, now) => {
                    Ok(Served {
                        body: body.clone(),
                        result: CacheResult::StaleOnError(reason),
                        age: age(*fetched_at, now),
                        revalidation: None,
                    })
                }
                _ => Err(ProxyError::Upstream(reason)),
            },
            Err(other) => Err(other),
        }
    }

    fn fly(&self, job: Job) -> Flight {
        let inner = Arc::clone(&self.0);
        self.0
            .flights
            .join(job.key.clone(), || lead(inner, job).boxed())
    }

    fn revalidation(&self, job: Job) -> Revalidation {
        let engine = self.clone();
        // `fly` внутри async-блока: до первого опроса полёт не стартует.
        Revalidation(Box::pin(async move { engine.fly(job).await.map(drop) }))
    }
}

/// Use cases дают Send-future: entry исполняет их на многопоточном tokio.
/// Проверка при компиляции и для любых адаптеров, а не только memory.
#[cfg(test)]
#[expect(
    dead_code,
    reason = "проверка типов: функцию не вызывают, её компилируют"
)]
fn use_case_futures_are_send<U, C, L, B>(
    engine: &ProxyEngine<U, C, L, B>,
    origin: &Origin,
    path: &UpstreamPath,
    policy: &MetadataPolicy,
) where
    U: Upstream + 'static,
    C: ProxyCache + 'static,
    L: FetchLeases + 'static,
    B: BlobStore + 'static,
{
    fn send<T: Send>(_: T) {}
    send(engine.metadata(origin, path, policy));
    send(engine.artifact(origin, path, None, &NegativePolicy::ARTIFACT));
}
