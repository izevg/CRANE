//! Proxy-движок: fetch-through кэш metadata и неизменяемых артефактов, не знающий форматов.
//!
//! Алгоритм — 10.2, зоны свежести и сроки — 11.1. Формат передаёт путь и политику, получает байты
//! и факты для журнала доступа; ссылки в документах переписывает сам формат.

mod flight;
mod lead;

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use bytes::Bytes;
use crane_domain::proxy::{
    EgressRefusal, Freshness, MetadataPolicy, NegativePolicy, Origin, UpstreamPath, age,
};
use crane_domain::registry::Sha256;
use crane_ports::proxy::{Cached, CachedBody, CachedState, FetchLeases, ProxyCache, Upstream};
use crane_ports::registry::{BlobError, BlobStore, ByteStream, Digests};
use crane_ports::system::Clock;
use futures_util::FutureExt;

use self::flight::{Failure, Flight, Flights, Invariant};
use self::lead::{Job, Target, lead};

/// Пределы движка. Умолчания — 10.2 (lease 30 с, ожидание 25 с) плюс защита памяти от гигантского документа.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProxyLimits {
    pub max_document_bytes: u64,
    pub lease: Duration,
    pub patience: Duration,
}

impl ProxyLimits {
    pub const DEFAULT: Self = Self {
        max_document_bytes: 64 * 1024 * 1024,
        lease: Duration::from_secs(30),
        patience: Duration::from_secs(25),
    };
}

/// Метка `result` журнала доступа (16.6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CacheResult {
    Hit,
    Miss,
    /// Отдано просроченное (stale-while-revalidate), обновление идёт в фоне.
    Stale,
    /// Upstream не ответил, отдана устаревшая запись (заголовок `Warning`).
    StaleOnError(UpstreamFailure),
}

/// Почему upstream не дал ответа. Отказ SSRF не прячется за stale-ответом: это событие для `SecOps`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpstreamFailure {
    Refused(EgressRefusal),
    Unavailable,
}

/// Ответ движка с фактами для журнала доступа и заголовков `Age`/`Warning`.
pub struct Served<T> {
    pub body: T,
    pub result: CacheResult,
    pub age: Duration,
    /// Фоновое обновление, которое entry обязан исполнить (`spawn`); `None`, если обновлять нечего.
    pub revalidation: Option<Revalidation>,
}

impl<T> fmt::Debug for Served<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Тело и future не Debug и не должны попадать в журналы.
        f.debug_struct("Served")
            .field("result", &self.result)
            .field("age", &self.age)
            .field("revalidation", &self.revalidation.is_some())
            .finish_non_exhaustive()
    }
}

/// Артефакт из CAS.
pub struct Fetched {
    pub digests: Digests,
    pub body: ByteStream,
}

impl fmt::Debug for Fetched {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fetched")
            .field("digests", &self.digests)
            .finish_non_exhaustive()
    }
}

/// Фоновая ревалидация. Ленивая: полёт начинается (или присоединяется к идущему) только при первом опросе,
/// поэтому брошенная ревалидация ничего не стоит и не плодит выборок.
pub struct Revalidation(Pin<Box<dyn Future<Output = Result<(), ProxyError>> + Send>>);

impl Future for Revalidation {
    type Output = Result<(), ProxyError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.as_mut().poll(cx)
    }
}

impl fmt::Debug for Revalidation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Revalidation").finish_non_exhaustive()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProxyError {
    #[error("нет у upstream")]
    NotFound,
    #[error("upstream отклонён политикой egress")]
    Refused(#[source] EgressRefusal),
    #[error("upstream недоступен")]
    UpstreamUnavailable,
    #[error("sha256 не совпал: ожидался {expected}, получен {actual}")]
    DigestMismatch { expected: Sha256, actual: Sha256 },
    #[error("документ upstream больше {limit} байт")]
    TooLarge { limit: u64 },
    /// `Arc`, потому что итог полёта общий для всех присоединившихся и должен быть `Clone`.
    #[error("ошибка бэкенда")]
    Backend(#[source] Arc<dyn std::error::Error + Send + Sync>),
}

fn invariant(what: &'static str) -> ProxyError {
    ProxyError::Backend(Arc::new(Invariant(what)))
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
        let key = origin.key(path);
        let now = self.0.clock.now();
        // Ошибка кэша — промах: кэш не источник истины.
        let cached = self.0.cache.get(&key).await.ok().flatten();
        if let Some(entry) = &cached {
            match &entry.state {
                CachedState::Found {
                    body: CachedBody::Document(body),
                    unchanged,
                    ..
                } => {
                    let entry_age = age(entry.fetched_at, now);
                    match policy.assess(*unchanged, entry.fetched_at, now) {
                        Freshness::Fresh => {
                            return Ok(Served {
                                body: body.clone(),
                                result: CacheResult::Hit,
                                age: entry_age,
                                revalidation: None,
                            });
                        }
                        Freshness::Revalidate => {
                            // Refresh-ahead (последние 10% TTL) ещё не «stale» для клиента, SWR уже да.
                            let result = if entry_age < policy.effective_ttl(*unchanged) {
                                CacheResult::Hit
                            } else {
                                CacheResult::Stale
                            };
                            let job = Job {
                                key,
                                base: origin.base.clone(),
                                path: path.clone(),
                                target: Target::Document { policy: *policy },
                                prev: cached.clone(),
                            };
                            return Ok(Served {
                                body: body.clone(),
                                result,
                                age: entry_age,
                                revalidation: Some(self.revalidation(job)),
                            });
                        }
                        Freshness::Expired => {}
                    }
                }
                CachedState::Missing { misses }
                    if !policy.negative.is_expired(*misses, entry.fetched_at, now) =>
                {
                    return Err(ProxyError::NotFound);
                }
                _ => {}
            }
        }
        let job = Job {
            key,
            base: origin.base.clone(),
            path: path.clone(),
            target: Target::Document { policy: *policy },
            prev: cached.clone(),
        };
        let outcome = self.fly(job).await;
        self.document_served(outcome, cached.as_ref(), policy)
    }

    /// Неизменяемый артефакт через CAS. Stale-if-error для артефактов нет (11.1): хит либо выборка.
    pub async fn artifact(
        &self,
        origin: &Origin,
        path: &UpstreamPath,
        expected: Option<Sha256>,
        negative: &NegativePolicy,
    ) -> Result<Served<Fetched>, ProxyError> {
        let key = origin.key(path);
        let now = self.0.clock.now();
        let cached = self.0.cache.get(&key).await.ok().flatten();
        let mut prev = cached.clone();
        match &cached {
            Some(Cached {
                fetched_at,
                state:
                    CachedState::Found {
                        body: CachedBody::Blob(digests),
                        ..
                    },
            }) if expected.is_none_or(|expected| expected == digests.sha256) => {
                match self.0.blobs.open(&digests.sha256).await {
                    Ok(body) => {
                        return Ok(Served {
                            body: Fetched {
                                digests: *digests,
                                body,
                            },
                            result: CacheResult::Hit,
                            age: age(*fetched_at, now),
                            revalidation: None,
                        });
                    }
                    // Класс proxy-cache разрешено терять (09): запись без blob — промах, качаем заново.
                    Err(BlobError::NotFound(_)) => prev = None,
                    Err(error) => return Err(ProxyError::Backend(Arc::new(error))),
                }
            }
            Some(Cached {
                fetched_at,
                state: CachedState::Missing { misses },
            }) if !negative.is_expired(*misses, *fetched_at, now) => {
                return Err(ProxyError::NotFound);
            }
            _ => {}
        }
        let job = Job {
            key,
            base: origin.base.clone(),
            path: path.clone(),
            target: Target::Artifact {
                expected,
                negative: *negative,
            },
            prev,
        };
        match self.fly(job).await? {
            Cached {
                fetched_at,
                state:
                    CachedState::Found {
                        body: CachedBody::Blob(digests),
                        ..
                    },
            } => {
                // Полёт общий, а ожидание у каждого своё: гарантию дайджеста держим для каждого вызывающего.
                if let Some(expected) = expected
                    && expected != digests.sha256
                {
                    return Err(ProxyError::DigestMismatch {
                        expected,
                        actual: digests.sha256,
                    });
                }
                let body = self
                    .0
                    .blobs
                    .open(&digests.sha256)
                    .await
                    .map_err(|error| ProxyError::Backend(Arc::new(error)))?;
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
                state:
                    CachedState::Found {
                        body: CachedBody::Document(_),
                        ..
                    },
                ..
            } => Err(invariant("под ключом артефакта лежит документ")),
        }
    }

    /// Итог блокирующей выборки документа; при сбое upstream просроченная запись ещё может выручить.
    fn document_served(
        &self,
        outcome: Result<Cached, Failure>,
        cached: Option<&Cached>,
        policy: &MetadataPolicy,
    ) -> Result<Served<Bytes>, ProxyError> {
        let now = self.0.clock.now();
        match outcome {
            Ok(Cached {
                fetched_at,
                state:
                    CachedState::Found {
                        body: CachedBody::Document(body),
                        ..
                    },
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
                state:
                    CachedState::Found {
                        body: CachedBody::Blob(_),
                        ..
                    },
                ..
            }) => Err(invariant("под ключом документа лежит артефакт")),
            Err(failure) => {
                let reason = match &failure {
                    Failure::Refused(refusal) => Some(UpstreamFailure::Refused(refusal.clone())),
                    Failure::Unavailable => Some(UpstreamFailure::Unavailable),
                    Failure::DigestMismatch { .. }
                    | Failure::TooLarge { .. }
                    | Failure::Backend(_) => None,
                };
                match (reason, cached) {
                    (
                        Some(reason),
                        Some(Cached {
                            fetched_at,
                            state:
                                CachedState::Found {
                                    body: CachedBody::Document(body),
                                    unchanged,
                                    ..
                                },
                        }),
                    ) if policy.usable_on_error(*unchanged, *fetched_at, now) => Ok(Served {
                        body: body.clone(),
                        result: CacheResult::StaleOnError(reason),
                        age: age(*fetched_at, now),
                        revalidation: None,
                    }),
                    _ => Err(failure.into()),
                }
            }
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
        Revalidation(Box::pin(async move {
            engine.fly(job).await.map(drop).map_err(ProxyError::from)
        }))
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
