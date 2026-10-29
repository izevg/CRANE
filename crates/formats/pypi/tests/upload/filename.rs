use crane_format_pypi::{UploadError, admit};

use super::{wheel_form, with};

#[test]
fn file_names_that_could_escape_the_storage_path_or_the_url_are_refused() {
    // Arrange
    let names = [
        "../acme_core-1.0.0-py3-none-any.whl",
        "..",
        ".",
        "dir/acme_core-1.0.0-py3-none-any.whl",
        "dir\\acme_core-1.0.0-py3-none-any.whl",
        "/etc/passwd",
        "C:\\acme_core-1.0.0-py3-none-any.whl",
        "acme_core-1.0.0-py3-none-any.whl/",
        "acme\0core-1.0.0-py3-none-any.whl",
        "acme\ncore-1.0.0-py3-none-any.whl",
        "acme core-1.0.0-py3-none-any.whl",
        "acme%2e%2e-1.0.0-py3-none-any.whl",
        "acme?-1.0.0-py3-none-any.whl",
        "acme#-1.0.0-py3-none-any.whl",
        "acmé_core-1.0.0-py3-none-any.whl",
    ];

    // Act
    let actual = names.map(|name| admit(&wheel_form(), name).err());

    // Assert
    assert_eq!(actual, [(); 15].map(|()| Some(UploadError::UnsafeFilename)));
}

#[test]
fn file_name_longer_than_a_file_system_allows_is_refused() {
    // Arrange
    let name = format!("{}-1.0.0-py3-none-any.whl", "a".repeat(250));

    // Act
    let result = admit(&with(wheel_form(), "name", &"a".repeat(250)), &name);

    // Assert
    assert_eq!(result.err(), Some(UploadError::UnsafeFilename));
}

#[test]
fn file_name_of_exactly_the_maximal_length_is_accepted() {
    // Arrange
    let project = "a".repeat(255 - "-1.0.0-py3-none-any.whl".len());
    let name = format!("{project}-1.0.0-py3-none-any.whl");

    // Act
    let result = admit(&with(wheel_form(), "name", &project), &name);

    // Assert
    assert_eq!((name.len(), result.is_ok()), (255, true));
}

#[test]
fn missing_file_name_is_refused() {
    // Arrange
    let fields = wheel_form();

    // Act
    let result = admit(&fields, "");

    // Assert
    assert_eq!(result.err(), Some(UploadError::EmptyFilename));
}

#[test]
fn epoch_and_local_version_characters_are_allowed_in_the_file_name() {
    // Arrange
    let fields = with(wheel_form(), "version", "1!2.0+cpu.1");

    // Act
    let result = admit(&fields, "Acme_Core-1!2.0+cpu.1-py3-none-any.whl");

    // Assert
    assert!(result.is_ok());
}
