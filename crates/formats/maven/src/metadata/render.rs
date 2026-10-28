//! Проводной формат `maven-metadata.xml`: документы плоские и фиксированной формы, XML только
//! пишется. Поэтому шаблон строится вручную, а текст экранируется одной функцией.

use std::fmt::Write;
use std::time::SystemTime;

use crate::ecosystem::Build;

const HEADER: &str =
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<metadata modelVersion=\"1.1.0\">\n";
const SECONDS_PER_DAY: u64 = 86_400;
const SECONDS_PER_HOUR: u64 = 3_600;
const SECONDS_PER_MINUTE: u64 = 60;

/// Факты уровня артефакта.
pub(super) struct VersionsDoc<'a> {
    pub group: &'a str,
    pub artifact: &'a str,
    pub latest: &'a str,
    pub release: Option<&'a str>,
    pub versions: &'a [String],
    pub last_updated: SystemTime,
}

/// Факты уровня SNAPSHOT-версии.
pub(super) struct BuildsDoc<'a> {
    pub group: &'a str,
    pub artifact: &'a str,
    /// `X-SNAPSHOT`.
    pub version: &'a str,
    pub latest: &'a Build<'a>,
    pub last_updated: SystemTime,
    pub files: Vec<SnapshotFile<'a>>,
}

/// Файл SNAPSHOT-версии в самой новой сборке, где он есть.
pub(super) struct SnapshotFile<'a> {
    pub classifier: Option<&'a str>,
    pub extension: &'a str,
    /// Версия сборки: `X-yyyyMMdd.HHmmss-N`.
    pub value: &'a str,
    pub updated: SystemTime,
}

pub(super) fn versions(doc: &VersionsDoc<'_>) -> String {
    let mut out = String::from(HEADER);
    leaf(&mut out, 1, "groupId", doc.group);
    leaf(&mut out, 1, "artifactId", doc.artifact);
    out.push_str("  <versioning>\n");
    leaf(&mut out, 2, "latest", doc.latest);
    if let Some(release) = doc.release {
        leaf(&mut out, 2, "release", release);
    }
    out.push_str("    <versions>\n");
    for version in doc.versions {
        leaf(&mut out, 3, "version", version);
    }
    out.push_str("    </versions>\n");
    leaf(&mut out, 2, "lastUpdated", &timestamp(doc.last_updated));
    out.push_str("  </versioning>\n</metadata>\n");
    out
}

pub(super) fn builds(doc: &BuildsDoc<'_>) -> String {
    let mut out = String::from(HEADER);
    leaf(&mut out, 1, "groupId", doc.group);
    leaf(&mut out, 1, "artifactId", doc.artifact);
    leaf(&mut out, 1, "version", doc.version);
    out.push_str("  <versioning>\n    <snapshot>\n");
    leaf(&mut out, 3, "timestamp", &doc.latest.timestamp());
    leaf(&mut out, 3, "buildNumber", &doc.latest.number.to_string());
    out.push_str("    </snapshot>\n");
    leaf(&mut out, 2, "lastUpdated", &timestamp(doc.last_updated));
    out.push_str("    <snapshotVersions>\n");
    for file in &doc.files {
        out.push_str("      <snapshotVersion>\n");
        if let Some(classifier) = file.classifier {
            leaf(&mut out, 4, "classifier", classifier);
        }
        leaf(&mut out, 4, "extension", file.extension);
        leaf(&mut out, 4, "value", file.value);
        leaf(&mut out, 4, "updated", &timestamp(file.updated));
        out.push_str("      </snapshotVersion>\n");
    }
    out.push_str("    </snapshotVersions>\n  </versioning>\n</metadata>\n");
    out
}

/// `<tag>значение</tag>` с отступом в два пробела на уровень.
fn leaf(out: &mut String, depth: usize, tag: &str, value: &str) {
    let indent = "  ".repeat(depth);
    // Запись в `String` не может завершиться ошибкой.
    let _ = writeln!(out, "{indent}<{tag}>{}</{tag}>", escape(value));
}

/// Модель Maven допускает в версии `&` и `'`, поэтому экранируются все пять символов.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}

/// `yyyyMMddHHmmss` в UTC. Время до эпохи даёт эпоху.
pub(super) fn timestamp(time: SystemTime) -> String {
    let seconds = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let (year, month, day) = civil_date(seconds / SECONDS_PER_DAY);
    let of_day = seconds % SECONDS_PER_DAY;
    let hour = of_day / SECONDS_PER_HOUR;
    let minute = of_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE;
    let second = of_day % SECONDS_PER_MINUTE;
    format!("{year:04}{month:02}{day:02}{hour:02}{minute:02}{second:02}")
}

/// Дата по числу дней от 1970-01-01 (алгоритм `civil_from_days` Говарда Хиннанта,
/// пролептический григорианский календарь). Возвращает год, месяц `1..=12`, день `1..=31`.
fn civil_date(days: u64) -> (u64, u64, u64) {
    // Сдвиг к 0000-03-01: високосный день года оказывается в его конце.
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
    (year, month, day)
}
