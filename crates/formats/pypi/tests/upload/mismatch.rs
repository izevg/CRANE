use crane_format_pypi::{UploadError, admit};

use super::{wheel_form, with};

fn mismatch(what: &'static str, in_file: &str, in_form: &str) -> UploadError {
    UploadError::Mismatch {
        what,
        in_file: in_file.to_owned(),
        in_form: in_form.to_owned(),
    }
}

#[test]
fn project_in_the_file_name_must_equal_the_form_project() {
    // Arrange
    let fields = with(wheel_form(), "name", "other");

    // Act
    let result = admit(&fields, "acme_core-1.0.0-py3-none-any.whl");

    // Assert
    assert_eq!(result.err(), Some(mismatch("проект", "acme_core", "other")));
}

#[test]
fn version_in_the_file_name_must_equal_the_form_version() {
    // Arrange
    let fields = with(wheel_form(), "version", "1.0.1");

    // Act
    let result = admit(&fields, "Acme_Core-1.0.0-py3-none-any.whl");

    // Assert
    assert_eq!(result.err(), Some(mismatch("версия", "1.0.0", "1.0.1")));
}

#[test]
fn project_with_a_longer_name_does_not_pass_as_a_prefix_match() {
    // Arrange
    let fields = with(wheel_form(), "name", "acme");

    // Act
    let result = admit(&fields, "acme_core-1.0.0-py3-none-any.whl");

    // Assert
    assert_eq!(result.err(), Some(mismatch("проект", "acme_core", "acme")));
}

#[test]
fn unparsable_version_in_the_file_name_is_a_version_mismatch() {
    // Arrange
    let fields = with(wheel_form(), "version", "1.0.0");

    // Act
    let result = admit(&fields, "Acme_Core-latest-py3-none-any.whl");

    // Assert
    assert_eq!(result.err(), Some(mismatch("версия", "latest", "1.0.0")));
}
