//! JSON-представление (PEP 691, поля PEP 700): API 1.1.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::{DistFile, Project, Yanked};

const API_VERSION: &str = "1.1";
const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_MINUTE: i64 = 60;

pub(super) fn render(project: &Project) -> String {
    let files: Vec<Value> = project.files.iter().map(file).collect();
    json!({
        "meta": {"api-version": API_VERSION},
        "name": project.name.normalized(),
        "versions": project.versions,
        "files": files,
    })
    .to_string()
}

fn file(file: &DistFile) -> Value {
    let mut entry = Map::new();
    entry.insert("filename".into(), json!(file.filename));
    entry.insert("url".into(), json!(file.url));
    entry.insert("hashes".into(), json!({"sha256": file.sha256.to_string()}));
    entry.insert("size".into(), json!(file.size));
    entry.insert("yanked".into(), yanked(&file.yanked));
    if let Some(requires_python) = &file.requires_python {
        entry.insert("requires-python".into(), json!(requires_python));
    }
    if let Some(time) = file.upload_time.and_then(upload_time) {
        entry.insert("upload-time".into(), json!(time));
    }
    Value::Object(entry)
}

/// `false`, `true` или причина строкой (PEP 592 в JSON); пустая причина — просто `true`.
fn yanked(yanked: &Yanked) -> Value {
    match yanked {
        Yanked::No => json!(false),
        Yanked::Yes(Some(reason)) if !reason.is_empty() => json!(reason),
        Yanked::Yes(_) => json!(true),
    }
}

/// `yyyy-mm-ddThh:mm:ss.ffffffZ` в UTC. Время до эпохи не представимо: поле опускается.
fn upload_time(time: SystemTime) -> Option<String> {
    let since = time.duration_since(UNIX_EPOCH).ok()?;
    let seconds = i64::try_from(since.as_secs()).ok()?;
    let (year, month, day) = civil_date(seconds.div_euclid(SECONDS_PER_DAY));
    let in_day = seconds.rem_euclid(SECONDS_PER_DAY);
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:06}Z",
        in_day / SECONDS_PER_HOUR,
        in_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
        in_day % SECONDS_PER_MINUTE,
        since.subsec_micros()
    ))
}

/// Дата по числу дней от 1970-01-01 (алгоритм `civil_from_days` Говарда Хиннанта): своя функция
/// дешевле зависимости ради одного формата времени.
fn civil_date(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
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
    (year_of_era + era * 400 + i64::from(month <= 2), month, day)
}
