//! Composition root и точка входа сервера: роли serve, all, migrate, healthcheck.
//!
//! Конфигурация: env поверх необязательного TOML (`config`), логи JSON в stdout (раздел 16.6).

mod config;
mod run;
mod system;

use std::error::Error;
use std::path::Path;
use std::process::ExitCode;

use tracing_subscriber::EnvFilter;

use crate::config::{Settings, Storage};

const USAGE: &str = "использование: crane <serve|all|migrate|healthcheck>";
const EXIT_USAGE: u8 = 2;

#[derive(Debug, PartialEq, Eq)]
enum Role {
    Serve,
    Migrate,
    Healthcheck,
}

impl Role {
    /// Ровно одна роль. `all` — синоним `serve`, пока нет воркера.
    fn parse(args: &[String]) -> Option<Self> {
        match args {
            [role] => match role.as_str() {
                "serve" | "all" => Some(Self::Serve),
                "migrate" => Some(Self::Migrate),
                "healthcheck" => Some(Self::Healthcheck),
                _ => None,
            },
            _ => None,
        }
    }
}

fn main() -> ExitCode {
    init_logging();
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(role) = Role::parse(&args) else {
        tracing::error!("{USAGE}");
        return ExitCode::from(EXIT_USAGE);
    };
    match execute(&role) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(error = %chain(error.as_ref()), "роль завершилась с ошибкой");
            ExitCode::FAILURE
        }
    }
}

fn execute(role: &Role) -> Result<(), run::RunError> {
    let read_file = |path: &Path| std::fs::read_to_string(path);
    // Окружение важнее файла TOML из CRANE_CONFIG; путь к файлу задаёт только окружение.
    let var = config::layered(|name: &str| std::env::var(name).ok(), &read_file)?;
    match role {
        Role::Serve => run::serve(Settings::from_env(&var, &read_file)?),
        // Миграциям токен и публичный адрес не нужны.
        Role::Migrate => run::migrate(Storage::from_env(&var, &read_file)?),
        // Только порт admin: токен, адрес и хранилище пробе не нужны.
        Role::Healthcheck => run::healthcheck(config::admin_port(&var)?),
    }
}

/// Цели журнала ИБ v0.1.0 (раздел 14.9): аудит и лог доступа. Уровень логов их не глушит:
/// оператор, снизивший шум через `RUST_LOG`, не должен молча потерять журнал безопасности.
const SECURITY_TARGETS: [&str; 2] = [crane_siem::AUDIT_TARGET, crane_http::ACCESS_TARGET];

/// Директивы целей ИБ идут последними и перекрывают те же цели из `RUST_LOG`.
fn log_filter(env: Option<&str>) -> EnvFilter {
    let base = env
        .filter(|value| EnvFilter::try_new(value).is_ok())
        .unwrap_or("info");
    let pinned = SECURITY_TARGETS.map(|target| format!("{target}=info"));
    EnvFilter::new(format!("{base},{}", pinned.join(",")))
}

fn init_logging() {
    let filter = log_filter(std::env::var("RUST_LOG").ok().as_deref());
    tracing_subscriber::fmt()
        .json()
        // Поля событий (в том числе плоские пути OCSF `class_uid`, `actor.user.uid`) лежат
        // на верхнем уровне строки, а не под `fields`: коллектору не нужен второй путь.
        .flatten_event(true)
        .with_env_filter(filter)
        .init();
}

fn chain(error: &(dyn Error + 'static)) -> String {
    let mut text = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn serve_and_all_select_the_serve_role() {
        // Arrange
        let serve = args(&["serve"]);
        let all = args(&["all"]);

        // Act
        let roles = [Role::parse(&serve), Role::parse(&all)];

        // Assert
        assert_eq!(roles, [Some(Role::Serve), Some(Role::Serve)]);
    }

    #[test]
    fn migrate_selects_the_migrate_role() {
        // Arrange
        let migrate = args(&["migrate"]);

        // Act
        let role = Role::parse(&migrate);

        // Assert
        assert_eq!(role, Some(Role::Migrate));
    }

    #[test]
    fn healthcheck_selects_the_healthcheck_role() {
        // Arrange
        let healthcheck = args(&["healthcheck"]);

        // Act
        let role = Role::parse(&healthcheck);

        // Assert
        assert_eq!(role, Some(Role::Healthcheck));
    }

    #[test]
    fn anything_but_exactly_one_known_role_is_rejected() {
        // Arrange
        let cases = [
            args(&[]),
            args(&["worker"]),
            args(&["serve", "migrate"]),
            args(&["healthcheck", "extra"]),
            args(&[""]),
        ];

        // Act
        let roles = cases.map(|case| Role::parse(&case));

        // Assert
        assert_eq!(roles, [None, None, None, None, None]);
    }

    /// Включена ли запись `info` для цели под заданным фильтром.
    fn info_enabled(filter: EnvFilter, target: &'static str) -> bool {
        let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();
        tracing::subscriber::with_default(subscriber, || match target {
            crane_siem::AUDIT_TARGET => {
                tracing::enabled!(target: crane_siem::AUDIT_TARGET, tracing::Level::INFO)
            }
            crane_http::ACCESS_TARGET => {
                tracing::enabled!(target: crane_http::ACCESS_TARGET, tracing::Level::INFO)
            }
            _ => tracing::enabled!(target: "crane::other", tracing::Level::INFO),
        })
    }

    #[test]
    fn security_journal_survives_any_rust_log() {
        // Arrange
        let envs = [
            Some("warn"),
            Some("crane::audit=off,crane::access=off"),
            Some("off"),
            Some("неразборчиво=="),
            None,
        ];

        // Act
        let enabled =
            envs.map(|env| SECURITY_TARGETS.map(|target| info_enabled(log_filter(env), target)));

        // Assert
        assert_eq!(enabled, [[true, true]; 5]);
    }

    #[test]
    fn rust_log_still_controls_other_targets() {
        // Arrange
        let env = Some("warn");

        // Act
        let enabled = info_enabled(log_filter(env), "crane::other");

        // Assert
        assert!(!enabled);
    }

    #[test]
    fn unparsable_rust_log_falls_back_to_info_instead_of_silencing_logs() {
        // Arrange
        let env = Some("неразборчиво==");

        // Act
        let enabled = info_enabled(log_filter(env), "crane::other");

        // Assert
        assert!(enabled);
    }

    #[test]
    fn chain_lists_causes() {
        // Arrange
        let error = std::io::Error::other("корень");

        // Act
        let text = chain(&error);

        // Assert
        assert_eq!(text, "корень");
    }
}
