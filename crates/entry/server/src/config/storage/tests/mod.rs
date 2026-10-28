#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use database::{NO_SCHEME, scheme_of};

use crate::config::tests::{env, error_of, no_files, with};
use crane_domain::settings::Secret;

#[test]
fn unsupported_database_names_scheme_but_never_the_secret() {
    // Arrange
    let pairs = with(&[
        (DATABASE_URL, "mysql://u:SECRET@h/db"),
        (BLOBS, "file:///b"),
    ]);

    // Act
    let error = error_of(&pairs);

    // Assert
    let message = error.to_string();
    assert_eq!(
        error,
        ConfigError::Unsupported {
            var: DATABASE_URL,
            scheme: "mysql".to_owned()
        }
    );
    assert!(message.contains("mysql"), "{message}");
    assert!(!message.contains("SECRET"), "{message}");
}

#[test]
fn unparsable_database_url_error_never_contains_the_password() {
    // Arrange
    // sqlx цитирует значение `sslmode` в тексте ошибки: пароль в нём покажет, что маскировка снята.
    let urls = [
        "postgres://u:pw@h/db?sslmode=SECRETPW",
        "postgres://u:SECRETPW@h:notaport/db",
    ];

    // Act
    let messages = urls.map(|url| {
        Database::parse(&Secret::new(url.to_owned()))
            .err()
            .map(|error| error.to_string())
    });

    // Assert
    let leaked = messages.map(|message| message.is_none_or(|text| text.contains("SECRETPW")));
    assert_eq!(leaked, [false; 2]);
}

#[test]
fn unsupported_blobs_scheme_is_named_without_the_rest_of_the_value() {
    // Arrange
    let pairs = with(&[
        (DATABASE_URL, "postgres://u:pw@h/db"),
        (BLOBS, "gcs://SECRETbucket?key=SECRETkey"),
    ]);

    // Act
    let message = error_of(&pairs).to_string();

    // Assert
    assert!(message.contains("gcs"), "{message}");
    assert!(!message.contains("SECRET"), "{message}");
}

#[test]
fn invalid_blobs_value_is_rejected_without_echoing_it() {
    // Arrange
    let pairs = with(&[(BLOBS, "SECRETvalue")]);

    // Act
    let message = error_of(&pairs).to_string();

    // Assert
    assert!(message.contains(BLOBS), "{message}");
    assert!(!message.contains("SECRET"), "{message}");
}

/// Форма результата для таблицы: сравнивать `Storage` целиком нельзя, секрет не `PartialEq`.
fn shape(result: Result<Storage, ConfigError>) -> String {
    match result {
        Ok(Storage::Memory) => "memory".to_owned(),
        Ok(Storage::Durable {
            database, blobs, ..
        }) => {
            format!(
                "durable {}/{} {}",
                database.options().get_host(),
                database.options().get_database().unwrap_or_default(),
                blobs.scheme()
            )
        }
        Err(error) => format!("error {error}"),
    }
}

#[test]
fn storage_mode_follows_the_pair_of_database_and_blobs() {
    // Arrange
    let cases = [
        ("memory", "memory"),
        ("postgres://u:pw@h/db", "file:///data/blobs"),
        (
            "postgresql://u:pw@h/db",
            "s3://bucket/p?endpoint=http://s3:8333",
        ),
        ("memory", "file:///data/blobs"),
        ("postgres://u:pw@h/db", "memory"),
    ];

    // Act
    let shapes = cases.map(|(database, blobs)| {
        shape(Storage::from_env(
            &env(&[(DATABASE_URL, database), (BLOBS, blobs)]),
            &no_files,
        ))
    });

    // Assert
    let mixed =
        "error CRANE_DATABASE_URL и CRANE_BLOBS: memory нельзя смешивать с постоянным хранилищем";
    assert_eq!(
        shapes,
        [
            "memory",
            "durable h/db file",
            "durable h/db s3",
            mixed,
            mixed
        ]
    );
}

#[test]
fn blobs_query_accepts_only_public_keys_and_names_the_rejected_one() {
    // Arrange
    let cases = [
        "s3://b?endpoint=http://x:8333&aws_allow_http=true",
        "s3://b?aws_server_side_encryption=aws:kms&aws_sse_kms_key_id=alias/crane",
        "s3://b?aws_secret_access_key=SECRETKEY",
        "s3://b?aws_session_token=SECRETKEY",
        "s3://b?aws_sse_customer_key_base64=SECRETKEY",
        "s3://b?endpoint=http://u:SECRETKEY@x:8333",
        "s3://b?foo=1",
    ];

    // Act
    let results = cases.map(|blobs| {
        blobs_url(blobs)
            .err()
            .map_or_else(|| "принято".to_owned(), |error| error.to_string())
    });

    // Assert
    let rejected = |key: &str| {
        format!(
            "CRANE_BLOBS: ключ `{key}` не из списка несекретных параметров; учётные данные и их \
             поставщики S3 задаются только переменными AWS_*"
        )
    };
    assert_eq!(
        results,
        [
            "принято".to_owned(),
            "принято".to_owned(),
            rejected("aws_secret_access_key"),
            rejected("aws_session_token"),
            rejected("aws_sse_customer_key_base64"),
            rejected("endpoint"),
            rejected("foo"),
        ]
    );
}

/// Читает «файлы секретов» из памяти: путь `/s/<имя>` отдаёт `<имя>`-значение.
fn s3_files(path: &Path) -> io::Result<String> {
    match path.to_str() {
        Some("/s/id") => Ok("AKID\n".to_owned()),
        Some("/s/secret") => Ok("SKEY\n".to_owned()),
        _ => Err(io::Error::from(io::ErrorKind::NotFound)),
    }
}

/// Наблюдаемый факт без значения ключа: есть ли он и какая ошибка.
fn key_state(pairs: &[(&str, &str)]) -> String {
    let base = [
        (DATABASE_URL, "postgres://u:pw@h/db"),
        (BLOBS, "s3://b?endpoint=http://s3:8333"),
    ];
    let all: Vec<_> = base.iter().chain(pairs).copied().collect();
    match Storage::from_env(&env(&all), &s3_files) {
        Ok(Storage::Durable { s3_key, .. }) => format!("key={}", s3_key.is_some()),
        Ok(Storage::Memory) => "memory".to_owned(),
        Err(error) => format!("error {error}"),
    }
}

#[test]
fn s3_key_comes_only_from_a_complete_pair_of_files() {
    // Arrange
    let both = [(S3_KEY_ID_FILE, "/s/id"), (S3_SECRET_FILE, "/s/secret")];
    let only_id = [(S3_KEY_ID_FILE, "/s/id")];
    let only_secret = [(S3_SECRET_FILE, "/s/secret")];
    let id_twice = [
        (S3_KEY_ID, "x"),
        (S3_KEY_ID_FILE, "/s/id"),
        (S3_SECRET_FILE, "/s/secret"),
    ];
    let flat = [(S3_KEY_ID, "x"), (S3_SECRET, "y")];

    // Act
    let states = [
        key_state(&[]),
        key_state(&flat),
        key_state(&both),
        key_state(&only_id),
        key_state(&only_secret),
        key_state(&id_twice),
    ];

    // Assert
    assert_eq!(
        states,
        [
            "key=false".to_owned(),
            "key=false".to_owned(),
            "key=true".to_owned(),
            format!("error {}", ConfigError::Missing { var: S3_SECRET }),
            format!("error {}", ConfigError::Missing { var: S3_KEY_ID }),
            format!(
                "error {}",
                ConfigError::Conflict {
                    first: S3_KEY_ID,
                    second: S3_KEY_ID_FILE
                }
            ),
        ]
    );
}

#[test]
fn s3_key_files_with_memory_storage_are_rejected() {
    // Arrange
    let source = env(&[
        (DATABASE_URL, "memory"),
        (BLOBS, "memory"),
        (S3_KEY_ID_FILE, "/s/id"),
        (S3_SECRET_FILE, "/s/secret"),
    ]);

    // Act
    let error = Storage::from_env(&source, &s3_files).err();

    // Assert
    assert_eq!(error, Some(ConfigError::S3KeyWithMemory));
}

#[test]
fn database_url_file_is_read_without_trailing_newline() {
    // Arrange
    let source = env(&[
        (DATABASE_URL_FILE, "/run/secrets/db"),
        (BLOBS, "file:///data/blobs"),
    ]);
    let read = |path: &Path| {
        (path == Path::new("/run/secrets/db"))
            .then(|| "postgres://u:pw@h/db\r\n".to_owned())
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    };

    // Act
    let storage = Storage::from_env(&source, &read);

    // Assert
    assert_eq!(shape(storage), "durable h/db file");
}

#[test]
fn database_url_and_its_file_together_conflict() {
    // Arrange
    let source = env(&[
        (DATABASE_URL, "memory"),
        (DATABASE_URL_FILE, "/db"),
        (BLOBS, "memory"),
    ]);

    // Act
    let error = Storage::from_env(&source, &no_files).err().unwrap();

    // Assert
    assert_eq!(
        error,
        ConfigError::Conflict {
            first: DATABASE_URL,
            second: DATABASE_URL_FILE
        }
    );
}

#[test]
fn scheme_of_weird_prefix_is_hidden() {
    // Arrange
    let value = "p@ss word://x";

    // Act
    let scheme = scheme_of(value);

    // Assert
    assert_eq!(scheme, NO_SCHEME);
}
