//! Composition root и точка входа сервера: роли serve, all, migrate, healthcheck.
//!
//! Конфигурация: env поверх необязательного TOML (`config`), логи JSON в stdout (раздел 16.6).

mod cache;
mod config;
mod ecosystem;
mod namespaces;
mod onboarding;
mod run;
mod system;

use std::error::Error;
use std::path::Path;
use std::process::ExitCode;

use tracing_subscriber::EnvFilter;

use crate::config::{Boot, Database};

const USAGE: &str = "использование: crane <serve|all|migrate|healthcheck|onboarding-token|onboarding-reset --confirm>";
const EXIT_USAGE: u8 = 2;
const CONFIRM_FLAG: &str = "--confirm";

#[derive(Debug, PartialEq, Eq)]
enum Role {
    Serve,
    Migrate,
    Healthcheck,
    /// Новый bootstrap-токен вместо потерянного; только пока мастер открыт.
    OnboardingToken,
    /// Break-glass: мастер открывается заново. Без `--confirm` отказ с подсказкой.
    OnboardingReset {
        confirmed: bool,
    },
}

impl Role {
    /// Ровно одна роль. `all` — синоним `serve`, пока нет воркера. Единственный флаг у
    /// `onboarding-reset`: `--confirm`.
    fn parse(args: &[String]) -> Option<Self> {
        match args {
            [role] => match role.as_str() {
                "serve" | "all" => Some(Self::Serve),
                "migrate" => Some(Self::Migrate),
                "healthcheck" => Some(Self::Healthcheck),
                "onboarding-token" => Some(Self::OnboardingToken),
                "onboarding-reset" => Some(Self::OnboardingReset { confirmed: false }),
                _ => None,
            },
            [role, flag] if role == "onboarding-reset" && flag == CONFIRM_FLAG => {
                Some(Self::OnboardingReset { confirmed: true })
            }
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
    let layered = config::layered(|name: &str| std::env::var(name).ok(), &read_file)?;
    let boot = Boot::new(layered, read_file);
    let inherited = |name: &str| boot.var(name);
    let database = || Database::from_env(&inherited, &|path: &Path| boot.read(path));
    match role {
        Role::Serve => run::serve(boot.clone()),
        // Миграциям токен, публичный адрес и blob не нужны.
        Role::Migrate => run::migrate(database()?),
        // Ролям первого запуска нужна только БД: полный Config и ключ шифрования не нужны.
        Role::OnboardingToken => onboarding::onboarding_token(database()?),
        Role::OnboardingReset { confirmed } => {
            onboarding::onboarding_reset(database()?, *confirmed)
        }
        // Только порт admin: токен, адрес и хранилище пробе не нужны.
        Role::Healthcheck => run::healthcheck(config::admin_port(&inherited)?),
    }
}

/// Цели журнала ИБ (раздел 14.9): аудит, лог доступа и egress. Уровень логов их не глушит:
/// оператор, снизивший шум через `RUST_LOG`, не должен молча потерять журнал безопасности.
const SECURITY_TARGETS: [&str; 3] = [
    crane_siem::AUDIT_TARGET,
    crane_http::ACCESS_TARGET,
    crane_upstream::EGRESS_TARGET,
];

/// Операционные цели, которые тоже нельзя заглушить `RUST_LOG`, но которые не журнал ИБ и не идут
/// в SIEM. `crane::bootstrap` несёт единственную печать bootstrap-токена (20.5): без неё
/// `RUST_LOG=warn` оставил бы оператора без входа в мастер.
const PINNED_TARGETS: [&str; 1] = [onboarding::BOOTSTRAP_TARGET];

/// Директивы закреплённых целей идут последними и перекрывают те же цели из `RUST_LOG`.
fn log_filter(env: Option<&str>) -> EnvFilter {
    let base = env
        .filter(|value| EnvFilter::try_new(value).is_ok())
        .unwrap_or("info");
    let pinned = SECURITY_TARGETS
        .iter()
        .chain(&PINNED_TARGETS)
        .map(|target| format!("{target}=info"))
        .collect::<Vec<_>>();
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
    fn onboarding_roles_take_only_the_confirm_flag_and_reset_refuses_without_it() {
        // Arrange
        let cases = [
            args(&["onboarding-token"]),
            args(&["onboarding-reset"]),
            args(&["onboarding-reset", "--confirm"]),
            args(&["onboarding-reset", "--yes"]),
            args(&["onboarding-token", "--confirm"]),
            args(&["serve", "--confirm"]),
        ];

        // Act
        let roles = cases.map(|case| Role::parse(&case));

        // Assert
        assert_eq!(
            roles,
            [
                Some(Role::OnboardingToken),
                Some(Role::OnboardingReset { confirmed: false }),
                Some(Role::OnboardingReset { confirmed: true }),
                None,
                None,
                None,
            ]
        );
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

    /// Заглушка точки вызова: нужна лишь для сборки `Metadata`, в реестр tracing не попадает.
    struct StubCallsite;

    static STUB: StubCallsite = StubCallsite;
    static STUB_META: tracing::Metadata<'static> = tracing::Metadata::new(
        "stub",
        "stub",
        tracing::Level::INFO,
        None,
        None,
        None,
        tracing::field::FieldSet::new(&[], tracing::callsite::Identifier(&STUB)),
        tracing::metadata::Kind::EVENT,
    );

    impl tracing::Callsite for StubCallsite {
        fn set_interest(&self, _: tracing::subscriber::Interest) {}

        fn metadata(&self) -> &tracing::Metadata<'_> {
            &STUB_META
        }
    }

    /// Включена ли запись `info` для цели под заданным фильтром.
    ///
    /// Спрашиваем `Subscriber::enabled` у собственного `Dispatch` напрямую. `tracing::enabled!`
    /// сначала читает глобальный кэш интереса статической точки вызова, общей для всех тестов
    /// с одной целью: параллельные `with_default` гоняются за его пересборкой, и тест видит
    /// чужой фильтр.
    fn info_enabled(filter: EnvFilter, target: &str) -> bool {
        let subscriber = tracing_subscriber::fmt().with_env_filter(filter).finish();
        let metadata = tracing::Metadata::new(
            "event",
            target,
            tracing::Level::INFO,
            None,
            None,
            None,
            tracing::field::FieldSet::new(&[], tracing::callsite::Identifier(&STUB)),
            tracing::metadata::Kind::EVENT,
        );
        tracing::Dispatch::new(subscriber).enabled(&metadata)
    }

    #[test]
    fn security_journal_survives_any_rust_log() {
        // Arrange
        let envs = [
            Some("warn"),
            Some("crane::audit=off,crane::access=off,crane::upstream=off"),
            Some("off"),
            Some("неразборчиво=="),
            None,
        ];

        // Act
        let enabled =
            envs.map(|env| SECURITY_TARGETS.map(|target| info_enabled(log_filter(env), target)));

        // Assert
        assert_eq!(enabled, [[true, true, true]; 5]);
    }

    #[test]
    fn bootstrap_token_line_survives_any_rust_log() {
        // Arrange
        let envs = [
            Some("warn"),
            Some("crane::bootstrap=off"),
            Some("off"),
            Some("неразборчиво=="),
            None,
        ];

        // Act
        let enabled = envs.map(|env| info_enabled(log_filter(env), onboarding::BOOTSTRAP_TARGET));

        // Assert
        assert_eq!(enabled, [true; 5]);
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
