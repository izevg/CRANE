//! Фикстуры `PyPI`: multipart как у twine, запросы с Basic-токеном, разбор страницы проекта.

use axum::http::{Method, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crane_domain::registry::Sha256 as Digest;
use sha2::{Digest as _, Sha256};

use crate::common::{Api, PYPI_REPO, Reply, TOKEN};

pub(crate) const BOUNDARY: &str = "----crane-test-boundary";
pub(crate) const WHEEL_BYTES: &[u8] = b"PK fake wheel";
pub(crate) const SDIST_BYTES: &[u8] = b"\x1f\x8b fake sdist";

pub(crate) fn upload_uri() -> String {
    format!("/pypi/{PYPI_REPO}/legacy/")
}

pub(crate) fn simple_uri(name: &str) -> String {
    format!("/pypi/{PYPI_REPO}/simple/{name}/")
}

pub(crate) fn file_uri(name: &str, version: &str, filename: &str) -> String {
    format!("/pypi/{PYPI_REPO}/files/{name}/{version}/{filename}")
}

pub(crate) fn yank_uri(name: &str, version: &str) -> String {
    format!("/pypi/{PYPI_REPO}/packages/{name}/versions/{version}/yank")
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    Digest::from(<[u8; 32]>::from(Sha256::digest(bytes))).to_string()
}

/// Basic как у pip, twine и uv: пользователь `__token__`, пароль — токен.
pub(crate) fn basic() -> String {
    format!("Basic {}", STANDARD.encode(format!("__token__:{TOKEN}")))
}

/// Файл для загрузки вместе с полями формы, которые шлёт twine.
pub(crate) struct Dist {
    pub(crate) name: &'static str,
    pub(crate) version: &'static str,
    pub(crate) filename: String,
    pub(crate) filetype: &'static str,
    pub(crate) bytes: Vec<u8>,
}

impl Dist {
    pub(crate) fn wheel(name: &'static str, version: &'static str) -> Self {
        Self {
            name,
            version,
            filename: format!("{}-{version}-py3-none-any.whl", name.replace('-', "_")),
            filetype: "bdist_wheel",
            bytes: WHEEL_BYTES.to_vec(),
        }
    }

    pub(crate) fn sdist(name: &'static str, version: &'static str) -> Self {
        Self {
            name,
            version,
            filename: format!("{}-{version}.tar.gz", name.replace('-', "_")),
            filetype: "sdist",
            bytes: SDIST_BYTES.to_vec(),
        }
    }

    pub(crate) fn fields(&self) -> Vec<(String, String)> {
        [
            (":action", "file_upload"),
            ("protocol_version", "1"),
            ("name", self.name),
            ("version", self.version),
            ("filetype", self.filetype),
            ("metadata_version", "2.1"),
            ("requires_python", ">=3.8"),
            ("classifiers", "A"),
            ("classifiers", "B"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .chain([("sha256_digest".to_owned(), sha256_hex(&self.bytes))])
        .collect()
    }

    /// Поля, затем файл: так шлют twine, poetry и uv.
    pub(crate) fn form(&self) -> Vec<u8> {
        multipart(&self.fields(), &self.filename, &self.bytes)
    }
}

fn part_header(name: &str, filename: Option<&str>) -> String {
    let file = filename.map_or_else(String::new, |f| format!("; filename=\"{f}\""));
    format!("--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"{file}\r\n\r\n")
}

pub(crate) fn multipart(fields: &[(String, String)], filename: &str, bytes: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(part_header(name, None).as_bytes());
        body.extend_from_slice(value.as_bytes());
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(part_header("content", Some(filename)).as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    body
}

pub(crate) fn form_content_type() -> String {
    format!("multipart/form-data; boundary={BOUNDARY}")
}

impl Api {
    /// Запрос с верным токеном и заданными заголовками.
    pub(crate) fn pypi(
        &self,
        method: Method,
        uri: &str,
        headers: &[(&str, &str)],
        body: Vec<u8>,
    ) -> Reply {
        let basic = basic();
        let mut all = vec![("authorization", basic.as_str())];
        all.extend_from_slice(headers);
        self.with_headers(method, uri, &all, body)
    }

    pub(crate) fn pypi_get(&self, uri: &str, accept: Option<&str>) -> Reply {
        let headers: Vec<(&str, &str)> = accept.map(|a| ("accept", a)).into_iter().collect();
        self.pypi(Method::GET, uri, &headers, Vec::new())
    }

    pub(crate) fn pypi_post(&self, uri: &str, body: Vec<u8>) -> Reply {
        let content_type = form_content_type();
        self.pypi(Method::POST, uri, &[("content-type", &content_type)], body)
    }

    pub(crate) fn pypi_upload(&self, dist: &Dist) -> Reply {
        self.pypi_post(&upload_uri(), dist.form())
    }

    /// Загрузка, которая обязана пройти: упавшая фикстура — не предмет теста.
    pub(crate) fn pypi_published(&self, dist: &Dist) {
        let reply = self.pypi_upload(dist);
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    }

    pub(crate) fn pypi_yank(&self, method: Method, name: &str, version: &str) -> Reply {
        self.pypi(method, &yank_uri(name, version), &[], Vec::new())
    }
}

pub(crate) const JSON_TYPE: &str = "application/vnd.pypi.simple.v1+json";

impl Reply {
    /// Тексты `<a …>` страницы проекта.
    pub(crate) fn anchors(&self) -> Vec<String> {
        self.text()
            .lines()
            .filter(|line| line.starts_with("<a "))
            .map(str::to_owned)
            .collect()
    }

    /// `detail` problem+json.
    pub(crate) fn problem_detail(&self) -> String {
        self.json()["detail"].as_str().unwrap().to_owned()
    }
}
