//! Загрузка legacy upload API: twine, poetry и uv шлют multipart с одним файлом `content`.

use std::io;
use std::sync::Arc;

use axum::body::Body;
use axum::extract::{Extension, Path, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use crane_app::registry::{Attach, Coordinate, RegistryError, Upload};
use crane_domain::access::Caller;
use crane_domain::registry::Format;
use crane_format_pypi::admit;
use crane_ports::registry::{BlobStore, Catalog};
use futures_util::TryStreamExt;
use multer::{Constraints, Field, Multipart, SizeLimit};

use super::Shared;
use crate::error::{ClientError, chain};

const FILE_FIELD: &str = "content";
/// Предел одного текстового поля: `description` (README пакета) бывает в сотни килобайт.
const TEXT_FIELD_LIMIT: u64 = 1024 * 1024;

pub(super) async fn upload<C: Catalog + 'static, B: BlobStore + 'static>(
    State(shared): State<Arc<Shared<C, B>>>,
    Extension(caller): Extension<Caller>,
    Path(repo): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, ClientError> {
    // Авторизация уже прошла в слое auth: при `Expect: 100-continue` клиент не шлёт тело отказанному.
    let limit = u64::try_from(shared.settings.max_publish_bytes).unwrap_or(u64::MAX);
    if declared_length(&headers).is_some_and(|length| length > limit) {
        return Err(too_large());
    }
    let boundary = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| multer::parse_boundary(value).ok())
        .ok_or_else(|| ClientError::bad_request("ожидается multipart/form-data"))?;
    // Размер всего тела ограничивает multer: сырой `Body` не знает про `DefaultBodyLimit`.
    let constraints = Constraints::new().size_limit(
        SizeLimit::new()
            .whole_stream(limit)
            .per_field(TEXT_FIELD_LIMIT)
            .for_field(FILE_FIELD, limit),
    );
    let mut form = Multipart::with_constraints(body.into_data_stream(), boundary, constraints);
    let mut fields = Vec::new();
    while let Some(field) = form.next_field().await.map_err(|e| form_error(&e))? {
        let Some(name) = field.name().map(str::to_owned) else {
            continue;
        };
        if name == FILE_FIELD {
            // Форма жива до конца `attach`: поток файла читает её же буфер.
            return attach_file(&shared, &caller, repo, &fields, field).await;
        }
        fields.push((name, field.text().await.map_err(|e| form_error(&e))?));
    }
    Err(ClientError::bad_request(
        "в форме нет части content с файлом",
    ))
}

/// Проверка по полям формы и имени файла идёт до чтения тела, поэтому часть `content` должна
/// стоять после `name` и `version` (так шлют twine, poetry и uv).
async fn attach_file<C: Catalog + 'static, B: BlobStore + 'static>(
    shared: &Shared<C, B>,
    caller: &Caller,
    repo: String,
    fields: &[(String, String)],
    file: Field<'static>,
) -> Result<Response, ClientError> {
    let filename = file.file_name().unwrap_or_default().to_owned();
    let accepted = admit(fields, &filename).map_err(|e| ClientError::bad_request(chain(&e)))?;
    let stored_as = accepted.filename.clone();
    let request = Attach {
        coordinate: Coordinate {
            format: Format::PyPi,
            repository: repo,
            package: accepted.package,
            version: accepted.version,
        },
        file: Upload {
            path: accepted.filename,
            body: Box::pin(file.map_err(io::Error::other)),
            declared: accepted.declared,
        },
        metadata: Ok(accepted.metadata),
    };
    // Успех и повтор того же файла одинаково отвечают 200.
    shared
        .registry
        .attach(caller, request)
        .await
        .map_err(|error| upload_error(error, &stored_as))?;
    Ok(StatusCode::OK.into_response())
}

fn declared_length(headers: &HeaderMap) -> Option<u64> {
    headers
        .get(header::CONTENT_LENGTH)?
        .to_str()
        .ok()?
        .parse()
        .ok()
}

fn too_large() -> ClientError {
    ClientError::new(
        StatusCode::PAYLOAD_TOO_LARGE,
        "загрузка превышает предел размера",
    )
}

fn is_size_exceeded(error: &multer::Error) -> bool {
    matches!(
        error,
        multer::Error::FieldSizeExceeded { .. } | multer::Error::StreamSizeExceeded { .. }
    )
}

/// Только 4xx: клиент (twine) повторяет любые 5xx.
fn form_error(error: &multer::Error) -> ClientError {
    if is_size_exceeded(error) {
        return too_large();
    }
    ClientError::bad_request(format!("некорректная форма загрузки: {error}"))
}

/// Другой файл под тем же именем — 409 с текстом `already exists`: по нему `--skip-existing`
/// twine и poetry узнают занятое имя файла. Превышение предела посреди файла приходит из
/// хранилища как обрыв тела: возвращаем 413.
fn upload_error(error: RegistryError, filename: &str) -> ClientError {
    if matches!(error, RegistryError::AssetExists) {
        return ClientError::new(
            StatusCode::CONFLICT,
            format!("File already exists: {filename}"),
        );
    }
    if let RegistryError::Body(source) = &error
        && source
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<multer::Error>())
            .is_some_and(is_size_exceeded)
    {
        return too_large();
    }
    error.into()
}
