#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

fn env_with(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
    let map: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect();
    move |key| map.get(key).cloned()
}

fn file(content: &'static str) -> impl Fn(&Path) -> io::Result<String> + use<> {
    move |path| {
        assert_eq!(path, Path::new("/etc/crane.toml"));
        Ok(content.to_owned())
    }
}

fn lookup(
    env: &[(&str, &str)],
    content: &'static str,
) -> Result<impl Fn(&str) -> Option<String> + use<>, ConfigError> {
    let mut all = vec![(CONFIG_PATH, "/etc/crane.toml")];
    all.extend_from_slice(env);
    layered(env_with(&all), &file(content))
}

fn error_of(env: &[(&str, &str)], content: &'static str) -> ConfigError {
    lookup(env, content).err().unwrap()
}

#[test]
fn without_config_path_there_is_no_file() {
    // Arrange
    let env = env_with(&[("CRANE_HTTP_PORT", "1")]);
    let never = |_: &Path| -> io::Result<String> {
        Err(io::Error::other("не должен читаться"))
    };

    // Act
    let get = layered(env, &never).ok().unwrap();

    // Assert
    assert_eq!(get("CRANE_HTTP_PORT").as_deref(), Some("1"));
    assert_eq!(get("CRANE_ADMIN_PORT"), None);
}

#[test]
fn file_value_is_used_when_env_is_absent() {
    // Arrange
    let get = lookup(&[], "public_url = \"https://file.example\"")
        .ok()
        .unwrap();

    // Act
    let value = get("CRANE_PUBLIC_URL");

    // Assert
    assert_eq!(value.as_deref(), Some("https://file.example"));
}

#[test]
fn env_overrides_file() {
    // Arrange
    let get = lookup(
        &[("CRANE_PUBLIC_URL", "https://env.example")],
        "public_url = \"https://file.example\"",
    )
    .ok()
    .unwrap();

    // Act
    let value = get("CRANE_PUBLIC_URL");

    // Assert
    assert_eq!(value.as_deref(), Some("https://env.example"));
}

#[test]
fn key_is_the_variable_name_without_prefix_in_lower_case() {
    // Arrange
    let get = lookup(
        &[],
        "token_principal = \"ci\"\nmax_publish_bytes = 3\ntrusted_proxies = \"10.0.0.0/8\"\n",
    )
    .ok()
    .unwrap();

    // Act
    let values = [
        "CRANE_TOKEN_PRINCIPAL",
        "CRANE_MAX_PUBLISH_BYTES",
        "CRANE_TRUSTED_PROXIES",
        "token_principal",
    ]
    .map(get);

    // Assert
    assert_eq!(
        values,
        [Some("ci"), Some("3"), Some("10.0.0.0/8"), None].map(|v| v.map(str::to_owned))
    );
}

#[test]
fn array_of_strings_becomes_comma_list_for_any_key() {
    // Arrange
    let get = lookup(
        &[],
        "cargo_repos = [\"a\", \"b\"]\ntrusted_proxies = [\"10.0.0.0/8\", \"::1\"]",
    )
    .ok()
    .unwrap();

    // Act
    let values = ["CRANE_CARGO_REPOS", "CRANE_TRUSTED_PROXIES"].map(get);

    // Assert
    assert_eq!(
        values,
        [Some("a,b".to_owned()), Some("10.0.0.0/8,::1".to_owned())]
    );
}

#[test]
fn unknown_key_is_rejected_by_name() {
    // Arrange
    let content = "colour = \"red\"";

    // Act
    let error = error_of(&[], content);

    // Assert
    assert_eq!(
        error,
        ConfigError::UnknownKey {
            key: "colour".to_owned()
        }
    );
}

#[test]
fn config_key_inside_the_file_is_rejected() {
    // Arrange
    let content = "config = \"/other.toml\"";

    // Act
    let error = error_of(&[], content);

    // Assert
    assert_eq!(
        error,
        ConfigError::UnknownKey {
            key: "config".to_owned()
        }
    );
}

#[test]
fn wrong_value_types_are_rejected_without_echoing_values() {
    // Arrange
    let cases = [
        "token = true",
        "http_port = 1.5",
        "cargo_repos = [\"a\", 1]",
        "token = \"SECRETVAL\"\nblobs = { a = \"SECRETVAL\" }",
    ];

    // Act
    let messages = cases.map(|content| error_of(&[], content).to_string());

    // Assert
    let scalar = "ожидалась строка, целое число или массив строк";
    assert_eq!(
        messages,
        [
            format!("некорректное значение ключа `token` в файле конфигурации: {scalar}"),
            format!("некорректное значение ключа `http_port` в файле конфигурации: {scalar}"),
            "некорректное значение ключа `cargo_repos` в файле конфигурации: ожидался массив строк"
                .to_owned(),
            format!("некорректное значение ключа `blobs` в файле конфигурации: {scalar}"),
        ]
    );
}

#[test]
fn parse_error_reports_position_but_not_file_contents() {
    // Arrange
    let content = "http_port = 1\ntoken = \"SECRETVAL\" bad\n";

    // Act
    let message = error_of(&[], content).to_string();

    // Assert
    assert_eq!(
        message,
        "ошибка разбора TOML в /etc/crane.toml: строка 2, столбец 21"
    );
}

#[test]
fn unreadable_file_reports_path_and_reason() {
    // Arrange
    let env = env_with(&[(CONFIG_PATH, "/missing.toml")]);
    let missing = |_: &Path| -> io::Result<String> {
        Err(io::Error::new(io::ErrorKind::NotFound, "нет файла"))
    };

    // Act
    let error = layered(env, &missing).err().unwrap();

    // Assert
    assert_eq!(
        error,
        ConfigError::FileRead {
            path: "/missing.toml".to_owned(),
            reason: "нет файла".to_owned()
        }
    );
}
