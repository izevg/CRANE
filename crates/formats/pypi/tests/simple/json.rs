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
fn upload_time_is_utc_microseconds_and_omitted_outside_years_1970_to_9999() {
    // Arrange
    let before_epoch = SystemTime::UNIX_EPOCH - Duration::from_secs(1);
    let last_second = at(253_402_300_799, 7);
    let year_10000 = at(253_402_300_800, 0);
    let times = [at(0, 7), last_second, year_10000, before_epoch];

    // Act
    let actual = times.map(|time| {
        let mut file = dist(WHEEL);
        file.upload_time = Some(time);
        json_of(&project(vec![file]))["files"][0]
            .get("upload-time")
            .cloned()
    });

    // Assert
    assert_eq!(
        actual,
        [
            Some(json!("1970-01-01T00:00:00.000007Z")),
            Some(json!("9999-12-31T23:59:59.000007Z")),
            None,
            None,
        ]
    );
}
