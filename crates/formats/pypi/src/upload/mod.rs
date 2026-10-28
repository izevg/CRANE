//! Legacy upload API (twine, poetry, uv publish): чистая проверка полей формы и имени файла.
//!
//! Entry читает multipart и отдаёт сюда текстовые поля и имя файла до чтения тела, поэтому
//! отказ стоит дешевле любого ввода-вывода.

mod filename;
mod md5;
mod metadata;

use crane_domain::registry::{
    Checksum, DigestError, FormatMetadata, Md5, NameError, PackageName, Sha256, Version,
    VersionError,
};

use crate::ecosystem::{normalize, parse_name, parse_version};
use filename::{Distribution, check_path, parse};
use md5::decode_md5;
pub(crate) use metadata::VersionMetadata;

const ACTION: &str = ":action";
const FILE_UPLOAD: &str = "file_upload";
const PROTOCOL_VERSION: &str = "protocol_version";

/// Принятая загрузка: всё, что нужно реестру, чтобы прикрепить файл.
#[derive(Debug)]
pub struct Accepted {
    pub package: PackageName,
    pub version: Version,
    pub filename: String,
    /// Заявленные `sha256_digest` и `md5_digest`: реестр сверяет каждую. `blake2_256_digest`
    /// принимается, но не сверяется: хранилище его не считает.
    pub declared: Vec<Checksum>,
    pub metadata: FormatMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UploadError {
    #[error("нет обязательного поля {0:?}")]
    MissingField(&'static str),
    #[error("поле :action должно быть file_upload, получено {0:?}")]
    Action(String),
    #[error("поле protocol_version должно быть 1, получено {0:?}")]
    Protocol(String),
    #[error("некорректное имя проекта")]
    Name(#[source] NameError),
    #[error("некорректная версия")]
    Version(#[source] VersionError),
    #[error("filetype {0:?} не поддерживается: принимаются bdist_wheel и sdist")]
    Filetype(String),
    #[error("у файла нет имени")]
    EmptyFilename,
    #[error("недопустимое имя файла: одно имя из латиницы, цифр и знаков . _ - + !")]
    UnsafeFilename,
    #[error("файл {0:?} не поддерживается: принимаются .whl и .tar.gz")]
    UnsupportedFile(String),
    #[error("filetype {filetype:?} не соответствует файлу {filename:?}")]
    FiletypeMismatch { filetype: String, filename: String },
    #[error("имя файла {0} не разобрано")]
    MalformedFilename(&'static str),
    #[error("{what} в имени файла ({in_file:?}) не совпадает с полем формы ({in_form:?})")]
    Mismatch {
        what: &'static str,
        in_file: String,
        in_form: String,
    },
    #[error("некорректное поле sha256_digest")]
    Digest(#[source] DigestError),
    #[error("некорректное поле md5_digest: ожидается urlsafe base64 без паддинга, 16 байт")]
    Md5Digest,
}

/// Поля формы в порядке прихода: повторы допустимы (`classifiers`, `requires_dist`).
struct Form<'a>(&'a [(String, String)]);

impl Form<'_> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    /// Непустое значение: пустая строка равна отсутствию (uv шлёт `requires_python` всегда).
    fn filled(&self, key: &str) -> Option<&str> {
        self.get(key)
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }

    fn required(&self, key: &'static str) -> Result<&str, UploadError> {
        self.filled(key).ok_or(UploadError::MissingField(key))
    }
}

/// Проверяет форму загрузки и имя файла.
pub fn admit(fields: &[(String, String)], filename: &str) -> Result<Accepted, UploadError> {
    let form = Form(fields);
    let action = form.required(ACTION)?;
    if action != FILE_UPLOAD {
        return Err(UploadError::Action(action.to_owned()));
    }
    let protocol = form.required(PROTOCOL_VERSION)?;
    if protocol != "1" {
        return Err(UploadError::Protocol(protocol.to_owned()));
    }
    check_path(filename)?;
    let package = parse_name(form.required("name")?).map_err(UploadError::Name)?;
    let version = parse_version(form.required("version")?).map_err(UploadError::Version)?;
    check_file(&form, filename, &package, &version)?;
    Ok(Accepted {
        package,
        version,
        filename: filename.to_owned(),
        declared: declared(&form)?,
        metadata: VersionMetadata {
            requires_python: form.filled("requires_python").map(str::to_owned),
            license_expression: form.filled("license_expression").map(str::to_owned),
        }
        .to_format(),
    })
}

/// `filetype` соответствует расширению, а проект и версия в имени файла — полям формы. Имя
/// сравнивается после нормализации (twine шлёт `safe_name`), версия — по каноническому виду.
fn check_file(
    form: &Form<'_>,
    filename: &str,
    package: &PackageName,
    version: &Version,
) -> Result<(), UploadError> {
    let file = parse(filename)?;
    let filetype = form.required("filetype")?;
    if Distribution::from_filetype(filetype)? != file.distribution {
        return Err(UploadError::FiletypeMismatch {
            filetype: filetype.to_owned(),
            filename: filename.to_owned(),
        });
    }
    if normalize(file.project) != package.normalized() {
        return Err(UploadError::Mismatch {
            what: "проект",
            in_file: file.project.to_owned(),
            in_form: package.as_written().to_owned(),
        });
    }
    let same_version = parse_version(file.version).is_ok_and(|in_file| in_file == *version);
    if !same_version {
        return Err(UploadError::Mismatch {
            what: "версия",
            in_file: file.version.to_owned(),
            in_form: version.as_written().to_owned(),
        });
    }
    Ok(())
}

fn declared(form: &Form<'_>) -> Result<Vec<Checksum>, UploadError> {
    let mut declared = Vec::new();
    if let Some(hex) = form.filled("sha256_digest") {
        let digest = Sha256::from_hex(hex).map_err(UploadError::Digest)?;
        declared.push(Checksum::Sha256(digest));
    }
    if let Some(encoded) = form.filled("md5_digest") {
        let bytes = decode_md5(encoded).ok_or(UploadError::Md5Digest)?;
        declared.push(Checksum::Md5(Md5::from(bytes)));
    }
    Ok(declared)
}
