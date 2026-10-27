//! Фикстуры cargo-протокола: раскладка индекса, тело публикации, строка индекса.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::Reply;

/// Раскладка индекса для коротких тестовых имён длиной >= 4.
pub(crate) fn index_path(name: &str) -> String {
    format!("{}/{}/{name}", &name[0..2], &name[2..4])
}

/// Тело `PUT /api/v1/crates/new`: длина JSON, JSON, длина крейта, крейт.
pub(crate) fn publish_body(name: &str, vers: &str, krate: &[u8]) -> Vec<u8> {
    let meta = json!({
        "name": name,
        "vers": vers,
        "deps": [{
            "name": "serde", "version_req": "^1.0", "features": ["derive"],
            "optional": false, "default_features": true, "target": null,
            "kind": "normal", "registry": null, "explicit_name_in_toml": null
        }],
        "features": { "default": ["std"], "std": [] },
        "description": "тестовый крейт",
        "license": "MIT"
    })
    .to_string();
    let mut out = Vec::new();
    out.extend_from_slice(&u32::try_from(meta.len()).unwrap().to_le_bytes());
    out.extend_from_slice(meta.as_bytes());
    out.extend_from_slice(&u32::try_from(krate.len()).unwrap().to_le_bytes());
    out.extend_from_slice(krate);
    out
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Единственная строка индекса как JSON.
pub(crate) fn single_index_line(reply: &Reply) -> Value {
    let text = reply.text();
    let mut lines = text.lines();
    let line = lines.next().unwrap();
    assert!(lines.next().is_none(), "ожидалась одна строка: {text}");
    serde_json::from_str(line).unwrap()
}
