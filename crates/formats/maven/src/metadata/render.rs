//! Проводной формат `maven-metadata.xml`: документы плоские и фиксированной формы, XML только
//! пишется. Поэтому шаблон строится вручную, а текст экранируется одной функцией.

use std::fmt::Write;
use std::time::SystemTime;

use time::OffsetDateTime;
use time::macros::format_description;

use crate::ecosystem::Build;

const HEADER: &str =
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<metadata modelVersion=\"1.1.0\">\n";

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

/// `yyyyMMddHHmmss` в UTC.
pub(super) fn timestamp(time: SystemTime) -> String {
    // Год вне 0000..=9999 формат не вмещает; такой `published_at` даёт пустую отметку.
    OffsetDateTime::from(time)
        .format(format_description!(
            "[year][month][day][hour][minute][second]"
        ))
        .unwrap_or_default()
}
