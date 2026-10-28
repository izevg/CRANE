//! `OnboardingStore` в PostgreSQL: единственная строка состояния мастера первого запуска.

use std::fmt;
use std::sync::Arc;
use std::time::SystemTime;

use crane_domain::access::Principal;
use crane_domain::onboarding::{Onboarding, TokenHash};
use crane_ports::settings::{OnboardingError, OnboardingStore};
use crane_ports::system::Clock;
use sqlx::PgPool;

use self::rows::{Columns, state_from_row};
use crate::db_time::to_db_time;
use crate::settings::BoxError;

mod rows;

const SELECT: &str =
    "SELECT token_hash, token_expires_at, completed_at, completed_by FROM onboarding";

// Конфликт по singleton - гонка реплик: строка уже есть, побеждает первая вставка.
const INSERT: &str =
    "INSERT INTO onboarding (token_hash, token_expires_at, completed_at, completed_by)
    VALUES ($1, $2, $3, $4) ON CONFLICT (singleton) DO NOTHING";

const ROTATE: &str = "UPDATE onboarding SET token_hash = $1, token_expires_at = $2
    WHERE completed_at IS NULL";

const RESET: &str = "UPDATE onboarding
    SET token_hash = $1, token_expires_at = $2, completed_at = NULL, completed_by = NULL";

const COMPLETE: &str = "UPDATE onboarding
    SET completed_at = $1, completed_by = $2, token_hash = NULL, token_expires_at = NULL
    WHERE completed_at IS NULL";

const OUT_OF_RANGE: &str = "момент времени вне диапазона timestamptz";

const EXISTS: &str = "SELECT EXISTS (SELECT 1 FROM onboarding)";

/// Единственная воронка ошибок адаптера: драйвер или описание рассинхрона схемы.
fn backend(error: impl Into<BoxError>) -> OnboardingError {
    OnboardingError::Backend(error.into())
}

/// Состояние мастера в PostgreSQL. Время завершения берёт из `Clock`.
/// Схему создаёт [`crate::migrate`].
pub struct PgOnboarding {
    pool: PgPool,
    clock: Arc<dyn Clock>,
}

impl fmt::Debug for PgOnboarding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // В строке хэш токена, поэтому содержимое не печатается.
        f.debug_struct("PgOnboarding").finish_non_exhaustive()
    }
}

impl PgOnboarding {
    #[must_use]
    pub const fn new(pool: PgPool, clock: Arc<dyn Clock>) -> Self {
        Self { pool, clock }
    }

    /// Почему переход не применился (ноль затронутых строк): строки нет или мастер завершён.
    async fn refusal(&self) -> OnboardingError {
        match sqlx::query_scalar::<_, bool>(EXISTS)
            .fetch_one(&self.pool)
            .await
        {
            Ok(true) => OnboardingError::Completed,
            Ok(false) => OnboardingError::NotStarted,
            Err(error) => backend(error),
        }
    }

    async fn set_token(
        &self,
        sql: &'static str,
        token: &TokenHash,
        expires_at: SystemTime,
    ) -> Result<u64, OnboardingError> {
        let expires_at = to_db_time(expires_at).ok_or_else(|| backend(OUT_OF_RANGE))?;
        sqlx::query(sql)
            .bind(token.as_bytes().to_vec())
            .bind(expires_at)
            .execute(&self.pool)
            .await
            .map(|done| done.rows_affected())
            .map_err(backend)
    }
}

impl OnboardingStore for PgOnboarding {
    async fn get(&self) -> Result<Option<Onboarding>, OnboardingError> {
        sqlx::query(SELECT)
            .fetch_optional(&self.pool)
            .await
            .map_err(backend)?
            .as_ref()
            .map(state_from_row)
            .transpose()
    }

    async fn begin(&self, initial: Onboarding) -> Result<Onboarding, OnboardingError> {
        let columns = Columns::encode(&initial)?;
        columns
            .bind_to(sqlx::query(INSERT))
            .execute(&self.pool)
            .await
            .map_err(backend)?;
        // Отдельный запрос, не `WITH ins AS (INSERT ..) SELECT ..`: в READ COMMITTED снимок
        // одного оператора взят до коммита конкурента и строки победителя не увидел бы.
        self.get()
            .await?
            .ok_or_else(|| backend("состояние первого запуска не читается после вставки"))
    }

    async fn rotate(
        &self,
        token: TokenHash,
        expires_at: SystemTime,
    ) -> Result<(), OnboardingError> {
        match self.set_token(ROTATE, &token, expires_at).await? {
            0 => Err(self.refusal().await),
            _ => Ok(()),
        }
    }

    async fn reset(&self, token: TokenHash, expires_at: SystemTime) -> Result<(), OnboardingError> {
        match self.set_token(RESET, &token, expires_at).await? {
            0 => Err(OnboardingError::NotStarted),
            _ => Ok(()),
        }
    }

    async fn complete(&self, by: &Principal) -> Result<(), OnboardingError> {
        let at = to_db_time(self.clock.now()).ok_or_else(|| backend(OUT_OF_RANGE))?;
        let affected = sqlx::query(COMPLETE)
            .bind(at)
            .bind(&by.id)
            .execute(&self.pool)
            .await
            .map_err(backend)?
            .rows_affected();
        match affected {
            0 => Err(self.refusal().await),
            _ => Ok(()),
        }
    }
}
