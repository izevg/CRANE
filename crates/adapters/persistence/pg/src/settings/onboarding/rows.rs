//! Отображение строки `onboarding` в доменное состояние и обратно. SQL здесь нет.

use crane_domain::onboarding::{Onboarding, TokenHash};
use crane_ports::settings::OnboardingError;
use sqlx::postgres::{PgArguments, PgRow};
use sqlx::query::Query;
use sqlx::{Postgres, Row};
use time::OffsetDateTime;

use super::{OUT_OF_RANGE, backend, driver};
use crate::db::{from_db_time, to_db_time};

/// Четыре колонки состояния в порядке `$1..$4` вставки.
pub(super) struct Columns {
    token_hash: Option<Vec<u8>>,
    token_expires_at: Option<OffsetDateTime>,
    completed_at: Option<OffsetDateTime>,
    completed_by: Option<String>,
}

impl Columns {
    pub(super) fn encode(state: &Onboarding) -> Result<Self, OnboardingError> {
        Ok(match state {
            Onboarding::Pending { token, expires_at } => Self {
                token_hash: Some(token.as_bytes().to_vec()),
                token_expires_at: Some(
                    to_db_time(*expires_at).ok_or_else(|| backend(OUT_OF_RANGE))?,
                ),
                completed_at: None,
                completed_by: None,
            },
            Onboarding::Completed { at, by } => Self {
                token_hash: None,
                token_expires_at: None,
                completed_at: Some(to_db_time(*at).ok_or_else(|| backend(OUT_OF_RANGE))?),
                completed_by: Some(by.clone()),
            },
        })
    }

    pub(super) fn bind_to(
        self,
        query: Query<'_, Postgres, PgArguments>,
    ) -> Query<'_, Postgres, PgArguments> {
        query
            .bind(self.token_hash)
            .bind(self.token_expires_at)
            .bind(self.completed_at)
            .bind(self.completed_by)
    }
}

/// Строка БД -> состояние. CHECK `onboarding_state` допускает только две формы строки;
/// любая другая значит, что схема разошлась с кодом.
pub(super) fn state_from_row(row: &PgRow) -> Result<Onboarding, OnboardingError> {
    let hash: Option<Vec<u8>> = row.try_get("token_hash").map_err(driver)?;
    let expires: Option<OffsetDateTime> = row.try_get("token_expires_at").map_err(driver)?;
    let at: Option<OffsetDateTime> = row.try_get("completed_at").map_err(driver)?;
    let by: Option<String> = row.try_get("completed_by").map_err(driver)?;
    match (hash, expires, at, by) {
        (Some(hash), Some(expires), None, None) => {
            let bytes: [u8; 32] = hash
                .try_into()
                .map_err(|_| backend("хэш токена первого запуска не 32 байта"))?;
            Ok(Onboarding::Pending {
                token: TokenHash::from_bytes(bytes),
                expires_at: from_db_time(expires)
                    .ok_or_else(|| backend("момент времени вне диапазона SystemTime"))?,
            })
        }
        (None, None, Some(at), Some(by)) => Ok(Onboarding::Completed {
            at: from_db_time(at)
                .ok_or_else(|| backend("момент времени вне диапазона SystemTime"))?,
            by,
        }),
        _ => Err(backend("строка первого запуска нарушает схему")),
    }
}
