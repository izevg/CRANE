//! Выпуск bootstrap-токена: первый старт, перевыпуск истёкшего и печать в `crane::bootstrap`.

use std::io;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crane_domain::onboarding::Onboarding;
use crane_ports::settings::{OnboardingError, OnboardingStore};
use crane_ports::system::Clock;

use super::OnboardingFailure;
use super::token::{BootstrapToken, RandomError};
use crate::config::{ConfigError, bootstrap_token, static_token_set};

/// Цель `tracing` с единственной печатью токена (20.5). Не входит в цели ИБ и SIEM: сборка
/// сервера держит её включённой при любом `RUST_LOG`, но поток аудита её не читает.
pub(crate) const BOOTSTRAP_TARGET: &str = "crane::bootstrap";

/// Срок жизни выпущенного токена; заданный оператором получает тот же срок в строке `onboarding`.
pub(crate) const TOKEN_TTL: Duration = Duration::from_hours(24);

/// Откуда берётся токен на этом старте.
pub(crate) enum Source {
    /// Задан `CRANE_TOKEN`: вход в мастер только по нему, bootstrap-токена нет вовсе.
    Static,
    /// Задан `CRANE_BOOTSTRAP_TOKEN` или `_FILE`: в лог не печатается.
    Provided(BootstrapToken),
    /// Процесс выпускает токен сам и печатает его.
    Generate,
}

impl Source {
    pub(crate) fn resolve(
        var: &impl Fn(&str) -> Option<String>,
        read_file: &impl Fn(&Path) -> io::Result<String>,
    ) -> Result<Self, ConfigError> {
        // Статический токен сильнее: bootstrap-токен принимается, только пока статического нет.
        if static_token_set(var) {
            return Ok(Self::Static);
        }
        Ok(
            bootstrap_token(var, read_file)?.map_or(Self::Generate, |secret| {
                Self::Provided(BootstrapToken::provided(secret))
            }),
        )
    }

    /// `Static` тоже получает случайный токен, но его никто не увидит: в строке `onboarding`
    /// остаётся хэш, не соответствующий ни одному выданному токену.
    pub(crate) fn mint(self) -> Result<Minted, RandomError> {
        match self {
            Self::Static => Ok(Minted::hidden(BootstrapToken::generate()?)),
            Self::Provided(token) => Ok(Minted::hidden(token)),
            Self::Generate => Ok(Minted {
                token: BootstrapToken::generate()?,
                show: true,
            }),
        }
    }
}

/// Токен и признак «печатать, если процесс победил».
pub(crate) struct Minted {
    token: BootstrapToken,
    show: bool,
}

impl Minted {
    const fn hidden(token: BootstrapToken) -> Self {
        Self { token, show: false }
    }
}

/// Токен, который нужно показать оператору, и срок его действия.
pub(crate) struct Issued {
    pub(crate) token: BootstrapToken,
    pub(crate) valid_until: SystemTime,
}

pub(crate) struct Begun {
    /// Строка `onboarding` содержит именно наш токен: процесс выиграл гонку реплик.
    pub(crate) won: bool,
    pub(crate) announce: Option<Issued>,
}

/// Первый старт без конфигурации: `begin(Pending)`. Победителя определяет `accepts`, а не флаг:
/// хранилище вернуло то, что лежит в строке, и принимает ли оно наш токен - единственный ответ.
pub(crate) async fn begin<S: OnboardingStore>(
    store: &S,
    clock: &dyn Clock,
    minted: Minted,
) -> Result<Begun, OnboardingError> {
    let now = clock.now();
    let expires_at = now + TOKEN_TTL;
    let state = store
        .begin(Onboarding::Pending {
            token: minted.token.hash(),
            expires_at,
        })
        .await?;
    let won = state.accepts(minted.token.expose().as_bytes(), now);
    let announce = (won && minted.show).then_some(Issued {
        token: minted.token,
        valid_until: expires_at,
    });
    Ok(Begun { won, announce })
}

/// Результат перевыпуска истёкшего токена.
pub(crate) struct Reissued {
    /// `None`, если токен задан оператором: печатать нечего.
    pub(crate) announce: Option<Issued>,
}

/// Истёкший токен в `Pending` заменяется на старте, если нет статического токена. Две реплики,
/// перевыпустившие его одновременно, напечатают по токену, действует последний (20.5).
pub(crate) async fn reissue_expired<S: OnboardingStore>(
    store: &S,
    clock: &dyn Clock,
    state: &Onboarding,
    source: Source,
) -> Result<Option<Reissued>, OnboardingFailure> {
    let now = clock.now();
    let Onboarding::Pending { expires_at, .. } = state else {
        return Ok(None);
    };
    if now < *expires_at || matches!(source, Source::Static) {
        return Ok(None);
    }
    let minted = source.mint()?;
    let valid_until = now + TOKEN_TTL;
    match store.rotate(minted.token.hash(), valid_until).await {
        // Мастер успели завершить или сбросить: цикл фаз прочитает новое состояние.
        Err(OnboardingError::Completed | OnboardingError::NotStarted) => return Ok(None),
        other => other?,
    }
    let announce = minted.show.then_some(Issued {
        token: minted.token,
        valid_until,
    });
    Ok(Some(Reissued { announce }))
}

/// Единственная печать токена. Ровно одно событие на выпуск; токен не форматируется нигде больше.
pub(crate) fn announce(issued: &Issued) {
    tracing::info!(
        target: BOOTSTRAP_TARGET,
        "bootstrap token: {} (valid until {})",
        issued.token.expose(),
        utc(issued.valid_until)
    );
}

const SECONDS_PER_DAY: u64 = 86_400;

/// RFC 3339 в UTC с точностью до секунды. Календарь по алгоритму `civil_from_days`
/// (Howard Hinnant): ради одной строки лога зависимость от `time` не нужна.
fn utc(time: SystemTime) -> String {
    let seconds = time.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let (days, rest) = (seconds / SECONDS_PER_DAY, seconds % SECONDS_PER_DAY);
    let shifted = days + 719_468;
    let era = shifted / 146_097;
    let day_of_era = shifted % 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + u64::from(month <= 2);
    let (hour, minute, second) = (rest / 3_600, rest % 3_600 / 60, rest % 60);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

#[cfg(test)]
#[path = "tests/bootstrap.rs"]
mod tests;
