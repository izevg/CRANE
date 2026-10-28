#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use bytes::Bytes;
use crane_domain::registry::FormatMetadata;
use crane_format_cargo::{MetadataError, PublishError, parse_publish};
use serde_json::{Value, json};

const OWN_INDEX: &str = "sparse+https://crane.example/cargo/internal/index/";
const CRATES_IO_GIT: &str = "https://github.com/rust-lang/crates.io-index";
const CRATES_IO_SPARSE: &str = "sparse+https://index.crates.io/";

fn realistic_json() -> Value {
    json!({
        "name": "My-Crate",
        "vers": "1.2.3",
        "deps": [
            {
                "name": "serde", "version_req": "^1.0", "features": ["derive"],
                "optional": false, "default_features": true, "target": null,
                "kind": "normal", "registry": "https://github.com/rust-lang/crates.io-index",
                "explicit_name_in_toml": null
            },
            {
                "name": "serde_json", "version_req": "^1", "features": [],
                "optional": true, "default_features": false, "target": "cfg(unix)",
                "kind": "build", "registry": null, "explicit_name_in_toml": "sj"
            },
            {
                "name": "pretty_assertions", "version_req": "^1", "features": [],
                "optional": false, "default_features": true, "target": null,
                "kind": "dev", "registry": null
            },
            { "name": "bare", "version_req": "*" }
        ],
        "features": { "default": ["std"], "std": [], "json": ["dep:serde_json", "serde/std"] },
        "links": "native",
        "rust_version": "1.74",
        "description": "Описание ".repeat(40),
        "documentation": "https://docs.example",
        "homepage": "https://home.example",
        "repository": "https://git.example/r",
        "license": "MIT",
        "license_file": null,
        "keywords": ["a", "b"],
        "categories": ["development-tools"],
        "readme": "# очень большой readme",
        "readme_file": "README.md",
        "authors": ["x"],
        "badges": {},
        "unknown_future_field": 1
    })
}

fn frame(json: &[u8], krate: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&u32::try_from(json.len()).unwrap().to_le_bytes());
    out.extend_from_slice(json);
    out.extend_from_slice(&u32::try_from(krate.len()).unwrap().to_le_bytes());
    out.extend_from_slice(krate);
    out
}

fn body_of(value: &Value, krate: &[u8]) -> Vec<u8> {
    frame(value.to_string().as_bytes(), krate)
}

fn stored(body: Vec<u8>) -> Value {
    let publish = parse_publish(Bytes::from(body)).unwrap();
    serde_json::from_str(publish.index_metadata(OWN_INDEX).unwrap().as_str()).unwrap()
}

#[test]
fn well_formed_body_yields_name_version_and_exact_crate_bytes() {
    // Arrange
    // JSON длиннее 255 байт: длина не помещается в один байт.
    let json = realistic_json();
    let krate = [0u8, 1, 2, 255, 254, 0, 7];

    // Act
    let publish = parse_publish(Bytes::from(body_of(&json, &krate))).unwrap();

    // Assert
    assert!(json.to_string().len() > 255, "условие теста: длинный JSON");
    assert_eq!(publish.name.as_written(), "My-Crate");
    assert_eq!(publish.version.as_written(), "1.2.3");
    assert_eq!(publish.crate_file.as_ref(), krate);
}

#[test]
fn empty_crate_is_accepted_by_the_parser() {
    // Arrange
    let body = body_of(&realistic_json(), &[]);

    // Act
    let publish = parse_publish(Bytes::from(body)).unwrap();

    // Assert
    assert!(publish.crate_file.is_empty());
}

#[test]
fn renamed_dependency_swaps_name_and_package() {
    // Arrange
    let body = body_of(&realistic_json(), b"x");

    // Act
    let meta = stored(body);

    // Assert
    let deps = &meta["index"]["deps"];
    assert_eq!(deps[1]["name"], "sj");
    assert_eq!(deps[1]["package"], "serde_json");
    assert_eq!(deps[0]["name"], "serde");
    assert_eq!(deps[0]["package"], Value::Null);
}

#[test]
fn dependency_fields_are_carried_over_with_defaults() {
    // Arrange
    let body = body_of(&realistic_json(), b"x");

    // Act
    let meta = stored(body);

    // Assert
    let deps = &meta["index"]["deps"];
    assert_eq!(
        deps[0],
        json!({
            "name": "serde", "req": "^1.0", "features": ["derive"], "optional": false,
            "default_features": true, "target": null, "kind": "normal",
            "registry": "https://github.com/rust-lang/crates.io-index", "package": null
        })
    );
    assert_eq!(deps[1]["target"], "cfg(unix)");
    assert_eq!(deps[1]["kind"], "build");
    assert_eq!(deps[1]["optional"], true);
    assert_eq!(deps[1]["default_features"], false);
    assert_eq!(deps[2]["kind"], "dev");
    // Пропущенные поля получают значения по умолчанию, отсутствующий kind -> normal.
    assert_eq!(
        deps[3],
        json!({
            "name": "bare", "req": "*", "features": [], "optional": false,
            "default_features": true, "target": null, "kind": "normal",
            "registry": null, "package": null
        })
    );
}

#[test]
fn features_links_and_rust_version_are_preserved() {
    // Arrange
    let body = body_of(&realistic_json(), b"x");

    // Act
    let meta = stored(body);

    // Assert
    assert_eq!(
        meta["index"]["features"],
        json!({"default": ["std"], "json": ["dep:serde_json", "serde/std"], "std": []})
    );
    assert_eq!(meta["index"]["links"], "native");
    assert_eq!(meta["index"]["rust_version"], "1.74");
}

#[test]
fn descriptive_fields_are_kept_and_readme_is_dropped() {
    // Arrange
    let body = body_of(&realistic_json(), b"x");

    // Act
    let meta = stored(body);

    // Assert
    let about = &meta["about"];
    assert_eq!(about["license"], "MIT");
    assert_eq!(about["homepage"], "https://home.example");
    assert_eq!(about["repository"], "https://git.example/r");
    assert_eq!(about["documentation"], "https://docs.example");
    assert_eq!(about["license_file"], Value::Null);
    assert_eq!(about["keywords"], json!(["a", "b"]));
    assert_eq!(about["categories"], json!(["development-tools"]));
    assert!(
        about["description"]
            .as_str()
            .unwrap()
            .starts_with("Описание")
    );
    assert!(about.get("readme").is_none());
}

#[test]
fn truncated_at_every_boundary_is_rejected() {
    // Arrange
    let full = body_of(&realistic_json(), b"crate-bytes");
    let json_len = full.len() - 4 - b"crate-bytes".len() - 4;
    let cuts = [
        0,                // нет заголовка длины JSON
        3,                // заголовок обрезан
        4 + json_len - 1, // JSON обрезан
        4 + json_len,     // нет заголовка длины крейта
        4 + json_len + 3, // заголовок крейта обрезан
        full.len() - 1,   // тело крейта обрезано
    ];

    // Act
    let results: Vec<bool> = cuts
        .iter()
        .map(|cut| {
            matches!(
                parse_publish(Bytes::copy_from_slice(&full[..*cut])),
                Err(PublishError::Truncated)
            )
        })
        .collect();

    // Assert
    assert_eq!(results, [true; 6]);
}

#[test]
fn bytes_after_the_crate_are_rejected() {
    // Arrange
    let mut body = body_of(&realistic_json(), b"abc");
    body.push(0);

    // Act
    let result = parse_publish(Bytes::from(body));

    // Assert
    assert!(
        matches!(result, Err(PublishError::TrailingBytes)),
        "{result:?}"
    );
}

#[test]
fn length_is_read_little_endian() {
    // Arrange
    // Длина JSON 0x0100 = 256 байт; при big-endian заголовок означал бы 65536.
    let json = format!(
        r#"{{"name":"a","vers":"1.0.0","description":"{}"}}"#,
        "x".repeat(256)
    );
    let body = frame(json.as_bytes(), b"z");

    // Act
    let publish = parse_publish(Bytes::from(body)).unwrap();

    // Assert
    assert_eq!(publish.crate_file.as_ref(), b"z");
}

#[test]
fn malformed_json_is_rejected() {
    // Arrange
    let body = frame(b"{not json", b"x");

    // Act
    let result = parse_publish(Bytes::from(body));

    // Assert
    assert!(matches!(result, Err(PublishError::Json(_))), "{result:?}");
}

#[test]
fn invalid_name_and_version_are_reported_separately() {
    // Arrange
    let bad_name = body_of(&json!({"name": "1bad", "vers": "1.0.0"}), b"x");
    let bad_version = body_of(&json!({"name": "good", "vers": "one"}), b"x");

    // Act
    let kinds = [bad_name, bad_version].map(|body| match parse_publish(Bytes::from(body)) {
        Err(PublishError::Name(_)) => "name",
        Err(PublishError::Version(_)) => "version",
        _ => "other",
    });

    // Assert
    assert_eq!(kinds, ["name", "version"]);
}

/// Тело публикации с одной зависимостью, у которой задан `registry`.
fn body_with_registry(registry: &Value) -> Vec<u8> {
    body_of(
        &json!({
            "name": "demo", "vers": "1.0.0",
            "deps": [{ "name": "dep", "version_req": "^1", "registry": registry }]
        }),
        b"x",
    )
}

/// Сохранённое поле `registry` либо признак отказа публикации.
fn registry_outcome(registry: &Value) -> Result<Value, &'static str> {
    let publish = parse_publish(Bytes::from(body_with_registry(registry))).unwrap();
    match publish.index_metadata(OWN_INDEX) {
        Ok(metadata) => {
            let meta: Value = serde_json::from_str(metadata.as_str()).unwrap();
            Ok(meta["index"]["deps"][0]["registry"].clone())
        }
        Err(MetadataError::Foreign(_)) => Err("rejected"),
        Err(MetadataError::Serialize(_)) => Err("other"),
    }
}

#[test]
fn dependency_registry_is_normalized_or_rejected() {
    // Arrange
    let cases = [
        Value::Null,
        json!(OWN_INDEX),
        json!(OWN_INDEX.trim_end_matches('/')),
        json!(CRATES_IO_GIT),
        json!(CRATES_IO_SPARSE),
        json!("sparse+https://evil.example/index/"),
        json!("sparse+https://crane.example/cargo/other/index/"),
        json!("https://crane.example/cargo/internal/index/"),
        json!(""),
    ];

    // Act
    let actual = cases.each_ref().map(registry_outcome);

    // Assert
    assert_eq!(
        actual,
        [
            Ok(Value::Null),
            Ok(Value::Null),
            Ok(Value::Null),
            Ok(json!(CRATES_IO_GIT)),
            Ok(json!(CRATES_IO_GIT)),
            Err("rejected"),
            Err("rejected"),
            Err("rejected"),
            Err("rejected"),
        ]
    );
}

#[test]
fn foreign_registry_error_clips_untrusted_values() {
    // Arrange
    let long = "x".repeat(1000);
    let publish = parse_publish(Bytes::from(body_with_registry(&json!(long)))).unwrap();

    // Act
    let result = publish.index_metadata(OWN_INDEX);

    // Assert
    assert_eq!(foreign_echo(result), Some(("dep".to_owned(), 128)));
}

/// Имя зависимости и длина `registry` из отказа; `None`, если отказа нет.
fn foreign_echo(result: Result<FormatMetadata, MetadataError>) -> Option<(String, usize)> {
    match result {
        Err(MetadataError::Foreign(foreign)) => {
            Some((foreign.dependency, foreign.registry.chars().count()))
        }
        _ => None,
    }
}
