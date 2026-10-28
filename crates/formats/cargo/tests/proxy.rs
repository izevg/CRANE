#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::registry::{Ecosystem, Sha256};
use crane_format_cargo::{Archive, CARGO, archive, index_path};

const SUM_A: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SUM_B: &str = "2222222222222222222222222222222222222222222222222222222222222222";

fn line(name: &str, vers: &str, cksum: &str) -> String {
    format!(
        r#"{{"name":"{name}","vers":"{vers}","deps":[],"cksum":"{cksum}","features":{{}},"yanked":false}}"#
    )
}

fn expected(path: &str, sum: &str) -> Archive {
    Archive {
        path: crane_domain::proxy::UpstreamPath::parse(path).unwrap(),
        sha256: Sha256::from_hex(sum).unwrap(),
    }
}

#[test]
fn archive_resolves_version_from_index_lines() {
    // Arrange
    let broken_sum = line("Inflector", "0.11.3", "zz");
    let garbage = "это не json";
    let valid = line("Inflector", "0.11.4", SUM_A);
    let duplicate = line("Inflector", "0.11.4", SUM_B);
    let index_with_missing = format!("{}\n", line("serde", "1.0.0", SUM_A));
    let cases: Vec<(String, &str)> = vec![
        // Имя пути берётся из строки индекса вместе с регистром.
        (format!("{valid}\n"), "0.11.4"),
        // Версии нет.
        (index_with_missing, "1.0.1"),
        // Битый cksum пропускается, как непарсящаяся строка; сосед найден.
        (format!("{garbage}\n{broken_sum}\n{valid}\n"), "0.11.4"),
        (format!("{garbage}\n{broken_sum}\n{valid}\n"), "0.11.3"),
        // Две строки одной версии: последняя.
        (format!("{valid}\n{duplicate}\n"), "0.11.4"),
        // Версия сравнивается побайтно, без нормализации.
        (format!("{valid}\n"), "0.11.4+build"),
        (String::new(), "0.11.4"),
    ];

    // Act
    let actual: Vec<Option<Archive>> = cases
        .iter()
        .map(|(index, version)| archive(index.as_bytes(), version))
        .collect();

    // Assert
    let path = "crates/Inflector/Inflector-0.11.4.crate";
    assert_eq!(
        actual,
        [
            Some(expected(path, SUM_A)),
            None,
            Some(expected(path, SUM_A)),
            None,
            Some(expected(path, SUM_B)),
            None,
            None,
        ]
    );
}

#[test]
fn archive_ignores_unknown_fields_and_escaped_strings() {
    // Arrange
    let index =
        format!(r#"{{"name":"a-b","vers":"1.0.0","cksum":"{SUM_A}","future":{{"x":[1,2]}}}}"#);

    // Act
    let actual = archive(index.as_bytes(), "1.0.0");

    // Assert
    assert_eq!(actual, Some(expected("crates/a-b/a-b-1.0.0.crate", SUM_A)));
}

#[test]
fn archive_rejects_name_that_would_escape_the_crates_directory() {
    // Arrange
    let index = line("../etc", "1.0.0", SUM_A);

    // Act
    let actual = archive(index.as_bytes(), "1.0.0");

    // Assert
    assert_eq!(actual, None);
}

#[test]
fn index_paths_never_share_first_segment_with_archive_paths() {
    // Arrange
    let names = [
        "a", "ab", "abc", "crates", "crate", "cratesio", "cr", "Crates",
    ];

    // Act
    let paths: Vec<String> = names
        .iter()
        .map(|raw| index_path(&CARGO.parse_name(raw).unwrap()))
        .collect();

    // Assert
    let clashes: Vec<&String> = paths
        .iter()
        .filter(|path| path.split('/').next() == Some("crates"))
        .collect();
    assert!(clashes.is_empty(), "{clashes:?}");
}

#[test]
fn index_negative_cache_grows_to_one_hour() {
    // Arrange
    let misses = 20;

    // Act
    let ttl = crane_format_cargo::INDEX.negative.ttl_after(misses);

    // Assert
    assert_eq!(ttl, std::time::Duration::from_hours(1));
}

#[test]
fn index_survives_upstream_errors_for_seven_days_after_ttl() {
    // Arrange
    let fetched_at = std::time::SystemTime::UNIX_EPOCH;
    let ttl = 5 * 60;
    let week = 7 * 24 * 60 * 60;
    let ages = [ttl + week - 1, ttl + week];

    // Act
    let actual = ages.map(|age| {
        let now = fetched_at + std::time::Duration::from_secs(age);
        crane_format_cargo::INDEX.usable_on_error(0, None, fetched_at, now)
    });

    // Assert
    assert_eq!(actual, [true, false]);
}
