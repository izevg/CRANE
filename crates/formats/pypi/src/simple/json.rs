//! JSON-представление (PEP 691, поля PEP 700): API 1.1.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Map, Value, json};

use super::{DistFile, Project, Yanked};

const API_VERSION: &str = "1.1";

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

/// Первая секунда 10000 года: `humantime` паникует на времени позже 9999-12-31 и раньше эпохи.
const YEAR_10000: u64 = 253_402_300_800;

/// `yyyy-mm-ddThh:mm:ss.ffffffZ` в UTC. Время вне 1970..9999 не представимо: поле опускается.
fn upload_time(time: SystemTime) -> Option<String> {
    let since = time.duration_since(UNIX_EPOCH).ok()?;
    (since.as_secs() < YEAR_10000).then(|| humantime::format_rfc3339_micros(time).to_string())
}
