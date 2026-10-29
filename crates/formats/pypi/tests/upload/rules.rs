use crane_domain::registry::{NameError, VersionError};
use crane_format_pypi::{UploadError, admit};

use super::{SDIST, WHEEL, wheel_form, with, without};

fn refusal(fields: &super::Fields, filename: &str) -> Option<UploadError> {
    admit(fields, filename).err()
}

#[test]
fn form_fields_that_break_the_protocol_are_refused_by_name() {
    // Arrange
    let cases = [
        without(wheel_form(), ":action"),
        with(wheel_form(), ":action", "submit"),
        without(wheel_form(), "protocol_version"),
        with(wheel_form(), "protocol_version", "2"),
        without(wheel_form(), "name"),
        without(wheel_form(), "version"),
        without(wheel_form(), "filetype"),
    ];

    // Act
    let actual = cases.map(|fields| refusal(&fields, WHEEL));

    // Assert
    assert_eq!(
        actual,
        [
            Some(UploadError::MissingField(":action")),
            Some(UploadError::Action("submit".to_owned())),
            Some(UploadError::MissingField("protocol_version")),
            Some(UploadError::Protocol("2".to_owned())),
            Some(UploadError::MissingField("name")),
            Some(UploadError::MissingField("version")),
            Some(UploadError::MissingField("filetype")),
        ]
    );
}

#[test]
fn invalid_project_name_or_version_in_the_form_is_refused_with_the_grammar_error() {
    // Arrange
    let bad_name = with(wheel_form(), "name", "-acme");
    let bad_version = with(wheel_form(), "version", "latest");

    // Act
    let actual = [refusal(&bad_name, WHEEL), refusal(&bad_version, WHEEL)];

    // Assert
    // Текст причины версии принадлежит pep440_rs: проверяем вид ошибки и исходную строку.
    assert!(
        matches!(
            &actual,
            [
                Some(UploadError::Name(NameError::InvalidStart('-'))),
                Some(UploadError::Version(VersionError::Invalid { raw, .. })),
            ] if raw == "latest"
        ),
        "{actual:?}"
    );
}

#[test]
fn only_wheels_and_tar_gz_sdists_are_accepted() {
    // Arrange
    let cases = [
        (
            with(wheel_form(), "filetype", "bdist_egg"),
            "acme_core-1.0.0-py3.11.egg",
        ),
        (
            with(wheel_form(), "filetype", "sdist"),
            "acme_core-1.0.0.zip",
        ),
        (
            with(wheel_form(), "filetype", "bdist_wininst"),
            "acme_core-1.0.0.win32.exe",
        ),
        (
            with(wheel_form(), "filetype", "sdist"),
            "acme_core-1.0.0.tar.bz2",
        ),
    ];

    // Act
    let actual = cases.map(|(fields, file)| refusal(&fields, file));

    // Assert
    assert_eq!(
        actual,
        [
            Some(UploadError::UnsupportedFile(
                "acme_core-1.0.0-py3.11.egg".to_owned()
            )),
            Some(UploadError::UnsupportedFile(
                "acme_core-1.0.0.zip".to_owned()
            )),
            Some(UploadError::UnsupportedFile(
                "acme_core-1.0.0.win32.exe".to_owned()
            )),
            Some(UploadError::UnsupportedFile(
                "acme_core-1.0.0.tar.bz2".to_owned()
            )),
        ]
    );
}

#[test]
fn filetype_must_match_the_file_extension() {
    // Arrange
    let wheel_as_sdist = with(wheel_form(), "filetype", "sdist");
    let sdist_as_wheel = wheel_form();
    let unknown = with(wheel_form(), "filetype", "bdist_dumb");

    // Act
    let actual = [
        refusal(&wheel_as_sdist, WHEEL),
        refusal(&sdist_as_wheel, SDIST),
        refusal(&unknown, WHEEL),
    ];

    // Assert
    assert_eq!(
        actual,
        [
            Some(UploadError::FiletypeMismatch {
                filetype: "sdist".to_owned(),
                filename: WHEEL.to_owned(),
            }),
            Some(UploadError::FiletypeMismatch {
                filetype: "bdist_wheel".to_owned(),
                filename: SDIST.to_owned(),
            }),
            Some(UploadError::Filetype("bdist_dumb".to_owned())),
        ]
    );
}

#[test]
fn malformed_wheel_and_sdist_names_are_refused() {
    // Arrange
    let sdist = with(wheel_form(), "filetype", "sdist");
    let cases = [
        (wheel_form(), "acme_core-1.0.0-py3-none.whl"),
        (wheel_form(), "acme_core-1.0.0-x-py3-none-any.whl"),
        (wheel_form(), "acme_core-1.0.0-1-2-py3-none-any.whl"),
        (wheel_form(), "acme_core--py3-none-any.whl"),
        (sdist, "acme_core.tar.gz"),
    ];

    // Act
    let actual = cases.map(|(fields, file)| refusal(&fields, file));

    // Assert
    assert_eq!(
        actual,
        [
            Some(UploadError::MalformedFilename("wheel")),
            Some(UploadError::MalformedFilename("wheel")),
            Some(UploadError::MalformedFilename("wheel")),
            Some(UploadError::MalformedFilename("wheel")),
            Some(UploadError::MalformedFilename("sdist")),
        ]
    );
}

#[test]
fn malformed_sha256_digest_is_refused() {
    // Arrange
    let cases = ["xyz", &"ab".repeat(31), &"ab".repeat(33)];

    // Act
    let actual = cases.map(|hex| {
        matches!(
            refusal(&with(wheel_form(), "sha256_digest", hex), WHEEL),
            Some(UploadError::Digest(_))
        )
    });

    // Assert
    assert_eq!(actual, [true; 3]);
}

#[test]
fn malformed_md5_digest_is_refused() {
    // Arrange
    // Hex, короткая и длинная строки, символ вне алфавита, стандартный алфавит с `+` и `/`.
    let cases = [
        "d41d8cd98f00b204e9800998ecf8427e",
        "1B2M2Y8AsgTpgAmY7PhCf",
        "1B2M2Y8AsgTpgAmY7PhCfgA",
        "1B2M2Y8AsgTpgAmY7Ph!fg",
        "+/+/+/+/+/+/+/+/+/+/+w",
    ];

    // Act
    let actual = cases.map(|md5| refusal(&with(wheel_form(), "md5_digest", md5), WHEEL));

    // Assert
    assert_eq!(actual, [(); 5].map(|()| Some(UploadError::Md5Digest)));
}
