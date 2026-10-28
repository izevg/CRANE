//! Загрузка legacy upload API: успех, повторы, отказы и пределы размера.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use bytes::Bytes;
use futures_util::stream;

use crate::common::{Api, MAX_PUBLISH_BYTES};
use crate::fixtures::{
    BOUNDARY, Dist, JSON_TYPE, basic, form_content_type, multipart, sha256_hex, simple_uri,
    upload_uri,
};

fn not_published(api: &Api) -> bool {
    api.pypi_get(&simple_uri("acme-core"), None).status == StatusCode::NOT_FOUND
}

#[test]
fn wheel_and_sdist_of_one_version_appear_as_two_files_of_the_project() {
    // Arrange
    let api = Api::new();
    api.pypi_published(&Dist::wheel("acme-core", "1.0.0"));

    // Act
    let reply = api.pypi_upload(&Dist::sdist("acme-core", "1.0"));

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    let page = api
        .pypi_get(&simple_uri("acme-core"), Some(JSON_TYPE))
        .json();
    let files: Vec<_> = page["files"]
        .as_array()
        .unwrap()
        .iter()
        .map(|file| file["filename"].as_str().unwrap())
        .collect();
    assert_eq!(
        (files, page["versions"].clone()),
        (
            vec!["acme_core-1.0.0-py3-none-any.whl", "acme_core-1.0.tar.gz"],
            serde_json::json!(["1"])
        )
    );
}

#[test]
fn repeating_the_same_file_is_accepted_and_changes_nothing() {
    // Arrange
    let api = Api::new();
    let dist = Dist::wheel("acme-core", "1.0.0");
    api.pypi_published(&dist);

    // Act
    let reply = api.pypi_upload(&dist);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
    let page = api.pypi_get(&simple_uri("acme-core"), None);
    assert_eq!(page.anchors().len(), 1);
}

#[test]
fn another_file_under_the_same_name_conflicts_with_the_text_skip_existing_looks_for() {
    // Arrange
    let api = Api::new();
    let mut dist = Dist::wheel("acme-core", "1.0.0");
    api.pypi_published(&dist);
    dist.bytes = b"other bytes".to_vec();

    // Act
    let reply = api.pypi_upload(&dist);

    // Assert
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(
        reply.problem_detail(),
        "File already exists: acme_core-1.0.0-py3-none-any.whl"
    );
}

#[test]
fn wrong_sha256_digest_is_a_client_error_and_publishes_nothing() {
    // Arrange
    let api = Api::new();
    let dist = Dist::wheel("acme-core", "1.0.0");
    let fields: Vec<_> = dist
        .fields()
        .into_iter()
        .map(|(key, value)| {
            if key == "sha256_digest" {
                (key, sha256_hex(b"something else"))
            } else {
                (key, value)
            }
        })
        .collect();
    let body = multipart(&fields, &dist.filename, &dist.bytes);

    // Act
    let reply = api.pypi_post(&upload_uri(), body);

    // Assert
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(not_published(&api));
}

/// md5 от `WHEEL_BYTES` в urlsafe base64 без паддинга, как его шлёт poetry.
const WHEEL_MD5: &str = "v5SfKiQu66AI0PP68LujNw";
/// md5 пустого файла: чужой дайджест.
const EMPTY_MD5: &str = "1B2M2Y8AsgTpgAmY7PhCfg";

fn upload_with_md5(api: &Api, md5: &str) -> crate::common::Reply {
    let dist = Dist::wheel("acme-core", "1.0.0");
    let mut fields = dist.fields();
    fields.push(("md5_digest".to_owned(), md5.to_owned()));
    api.pypi_post(
        &upload_uri(),
        multipart(&fields, &dist.filename, &dist.bytes),
    )
}

#[test]
fn matching_md5_digest_is_accepted() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = upload_with_md5(&api, WHEEL_MD5);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}

#[test]
fn wrong_md5_digest_is_a_client_error_and_publishes_nothing() {
    // Arrange
    let api = Api::new();

    // Act
    let reply = upload_with_md5(&api, EMPTY_MD5);

    // Assert
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(not_published(&api));
}

#[test]
fn both_spellings_of_the_upload_url_are_served_without_a_redirect() {
    // Arrange
    let api = Api::new();
    let with_slash = Dist::wheel("acme-core", "1.0");
    let without_slash = Dist::wheel("acme-core", "2.0");

    // Act
    let statuses = [
        api.pypi_post("/pypi/py/legacy/", with_slash.form()).status,
        api.pypi_post("/pypi/py/legacy", without_slash.form())
            .status,
    ];

    // Assert
    assert_eq!(statuses, [StatusCode::OK; 2]);
}

#[test]
fn form_that_does_not_match_the_file_is_a_bad_request_and_publishes_nothing() {
    // Arrange
    let api = Api::new();
    let mut dist = Dist::wheel("acme-core", "1.0.0");
    dist.filename = "other_name-1.0.0-py3-none-any.whl".to_owned();

    // Act
    let reply = api.pypi_upload(&dist);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert!(
        reply.problem_detail().contains("не совпадает"),
        "{}",
        reply.text()
    );
    assert!(not_published(&api));
}

#[test]
fn file_part_before_name_and_version_is_refused_without_buffering_it() {
    // Arrange
    let api = Api::new();
    let dist = Dist::wheel("acme-core", "1.0.0");
    let body = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"content\"; filename=\"{}\"\r\n\r\nbytes\r\n--{BOUNDARY}--\r\n",
        dist.filename
    );

    // Act
    let reply = api.pypi_post(&upload_uri(), body.into_bytes());

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
}

#[test]
fn form_without_a_file_part_and_a_body_that_is_not_multipart_are_bad_requests() {
    // Arrange
    let api = Api::new();
    let no_file = format!(
        "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nx\r\n--{BOUNDARY}--\r\n"
    );
    let json = [("content-type", "application/json")];

    // Act
    let statuses = [
        api.pypi_post(&upload_uri(), no_file.into_bytes()).status,
        api.pypi(Method::POST, &upload_uri(), &json, b"{}".to_vec())
            .status,
    ];

    // Assert
    assert_eq!(statuses, [StatusCode::BAD_REQUEST; 2]);
}

#[test]
fn body_cut_off_in_the_middle_of_the_file_is_a_client_error_not_a_server_one() {
    // Arrange
    let api = Api::new();
    let full = Dist::wheel("acme-core", "1.0.0").form();
    let cut = full[..full.len() - 40].to_vec();

    // Act
    let reply = api.pypi_post(&upload_uri(), cut);

    // Assert
    assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    assert!(not_published(&api));
}

#[test]
fn file_larger_than_the_limit_is_refused_while_streaming() {
    // Arrange
    let api = Api::new();
    let mut dist = Dist::wheel("acme-core", "1.0.0");
    dist.bytes = vec![7; MAX_PUBLISH_BYTES];
    let fields: Vec<_> = dist
        .fields()
        .into_iter()
        .filter(|(key, _)| key != "sha256_digest")
        .collect();
    let body = multipart(&fields, &dist.filename, &dist.bytes);

    // Act
    let reply = api.pypi_post(&upload_uri(), body);

    // Assert
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
    assert!(not_published(&api));
}

#[test]
fn declared_content_length_over_the_limit_is_refused_before_reading() {
    // Arrange
    let api = Api::new();
    let length = (MAX_PUBLISH_BYTES + 1).to_string();
    let content_type = form_content_type();

    // Act
    let reply = api.pypi(
        Method::POST,
        &upload_uri(),
        &[("content-type", &content_type), ("content-length", &length)],
        Vec::new(),
    );

    // Assert
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn request_exactly_at_the_limit_is_accepted() {
    // Arrange
    let api = Api::new();
    let mut dist = Dist::wheel("acme-core", "1.0.0");
    let overhead = dist.form().len() - dist.bytes.len();
    dist.bytes = vec![7; MAX_PUBLISH_BYTES - overhead];
    let body = dist.form();
    let length = body.len().to_string();
    let content_type = form_content_type();

    // Act
    let reply = api.pypi(
        Method::POST,
        &upload_uri(),
        &[("content-type", &content_type), ("content-length", &length)],
        body,
    );

    // Assert
    assert_eq!(
        (length.parse::<usize>().unwrap(), reply.status),
        (MAX_PUBLISH_BYTES, StatusCode::OK)
    );
}

#[test]
fn large_description_field_does_not_hit_the_text_field_limit() {
    // Arrange
    let api = Api::new();
    let dist = Dist::wheel("acme-core", "1.0.0");
    let mut fields = dist.fields();
    fields.push(("description".to_owned(), "d".repeat(32 * 1024)));
    let body = multipart(&fields, &dist.filename, &dist.bytes);

    // Act
    let reply = api.pypi_post(&upload_uri(), body);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}

#[test]
fn form_with_too_many_text_fields_is_refused_before_the_whole_body_is_read() {
    // Arrange
    let api = Api::new();
    let chunks: Vec<Bytes> = (0..1000)
        .map(|n| {
            Bytes::from(format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"f{n}\"\r\n\r\nv\r\n"
            ))
        })
        .collect();
    let total = chunks.len();
    let read = Arc::new(AtomicUsize::new(0));
    let counter = Arc::clone(&read);
    // Поток отдаёт чанки по одному и между ними уступает исполнителю, как сокет: готовый поток
    // multer вычитал бы целиком, не дойдя до проверки полей.
    let body = Body::from_stream(stream::unfold(chunks.into_iter(), move |mut rest| {
        let counter = Arc::clone(&counter);
        async move {
            tokio::task::yield_now().await;
            let chunk = rest.next()?;
            counter.fetch_add(1, Ordering::SeqCst);
            Some((Ok::<_, std::io::Error>(chunk), rest))
        }
    }));
    let request = Request::builder()
        .method(Method::POST)
        .uri(upload_uri())
        .header("authorization", basic())
        .header("content-type", form_content_type())
        .body(body)
        .unwrap();

    // Act
    let reply = api.send(request);

    // Assert
    assert_eq!(
        (reply.status, read.load(Ordering::SeqCst) < total * 3 / 4),
        (StatusCode::PAYLOAD_TOO_LARGE, true)
    );
}
