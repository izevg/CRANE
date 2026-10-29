//! Пределы текстовых полей формы загрузки: число, объём, части без имени.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use bytes::Bytes;
use futures_util::stream;

use crate::common::Api;
use crate::fixtures::{BOUNDARY, Dist, basic, form_content_type, multipart, upload_uri};

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

/// Часть формы без имени: `Content-Disposition: form-data` без `name`.
fn nameless_part() -> String {
    format!("--{BOUNDARY}\r\nContent-Disposition: form-data\r\n\r\nv\r\n")
}

#[test]
fn a_nameless_part_is_skipped_and_the_upload_still_succeeds() {
    // Arrange
    let api = Api::new();
    let dist = Dist::wheel("acme-core", "1.0.0");
    let mut body = nameless_part().into_bytes();
    body.extend(dist.form());

    // Act
    let reply = api.pypi_post(&upload_uri(), body);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}

#[test]
fn nameless_parts_count_toward_the_field_limit() {
    // Arrange
    let api = Api::new();
    let dist = Dist::wheel("acme-core", "1.0.0");
    let mut body = nameless_part().repeat(300).into_bytes();
    body.extend(dist.form());

    // Act
    let reply = api.pypi_post(&upload_uri(), body);

    // Assert
    assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE);
}

#[test]
fn field_whose_name_and_value_are_long_but_sum_to_little_is_accepted() {
    // Arrange
    let api = Api::new();
    let dist = Dist::wheel("acme-core", "1.0.0");
    let mut fields = dist.fields();
    fields.push(("n".repeat(2000), "v".repeat(2500)));
    let body = multipart(&fields, &dist.filename, &dist.bytes);

    // Act
    let reply = api.pypi_post(&upload_uri(), body);

    // Assert
    assert_eq!(reply.status, StatusCode::OK);
}
