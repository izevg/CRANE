use std::time::{Duration, SystemTime};

use crane_format_pypi::{View, Yanked, render};
use serde_json::{Value, json};

use super::{dist, project};

const WHEEL: &str = "acme_core-1.0-py3-none-any.whl";

fn json_of(project: &crane_format_pypi::Project) -> Value {
    serde_json::from_str(&render(project, View::Json)).unwrap()
}

fn at(seconds: u64, micros: u32) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::new(seconds, micros * 1000)
}

#[test]
fn page_declares_api_1_1_with_normalized_name_versions_and_file_fields() {
    // Arrange
    let mut file = dist(WHEEL);
    file.requires_python = Some(">=3.8".to_owned());
    file.upload_time = Some(at(1_700_000_000, 123_456));

    // Act
    let page = json_of(&project(vec![file]));

    // Assert
    assert_eq!(
        page,
        json!({
            "meta": {"api-version": "1.1"},
            "name": "acme-core",
            "versions": ["1"],
            "files": [{
                "filename": WHEEL,
                "url": format!("https://crane.example/pypi/internal/files/acme/1/{WHEEL}"),
                "hashes": {"sha256": "ab".repeat(32)},
                "requires-python": ">=3.8",
                "yanked": false,
                "size": 1234,
                "upload-time": "2023-11-14T22:13:20.123456Z",
            }],
        })
    );
}

#[test]
fn optional_fields_are_omitted_when_unknown() {
    // Arrange
    let file = dist(WHEEL);

    // Act
    let page = json_of(&project(vec![file]));

    // Assert
    let entry = page["files"][0].as_object().unwrap();
    assert_eq!(
        (
            entry.contains_key("requires-python"),
            entry.contains_key("upload-time")
        ),
        (false, false)
    );
}

#[test]
fn yanked_is_false_true_or_the_reason() {
    // Arrange
    let states = [
        Yanked::No,
        Yanked::Yes(None),
        Yanked::Yes(Some("security".to_owned())),
        Yanked::Yes(Some(String::new())),
    ];

    // Act
    let actual = states.map(|yanked| {
        let mut file = dist(WHEEL);
        file.yanked = yanked;
        json_of(&project(vec![file]))["files"][0]["yanked"].clone()
    });

    // Assert
    assert_eq!(
        actual,
        [json!(false), json!(true), json!("security"), json!(true)]
    );
}

#[test]
fn upload_time_is_utc_microseconds_across_leap_days_and_year_ends() {
    // Arrange
    let seconds = [0, 951_782_400, 1_709_164_800, 1_704_067_199, 4_102_444_800];

    // Act
    let actual = seconds.map(|s| {
        let mut file = dist(WHEEL);
        file.upload_time = Some(at(s, 7));
        json_of(&project(vec![file]))["files"][0]["upload-time"].clone()
    });

    // Assert
    assert_eq!(
        actual,
        [
            "1970-01-01T00:00:00.000007Z",
            "2000-02-29T00:00:00.000007Z",
            "2024-02-29T00:00:00.000007Z",
            "2023-12-31T23:59:59.000007Z",
            "2100-01-01T00:00:00.000007Z",
        ]
        .map(Value::from)
    );
}
