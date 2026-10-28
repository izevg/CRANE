use crane_format_pypi::{View, content_type};

const PIP: &str = "application/vnd.pypi.simple.v1+json, application/vnd.pypi.simple.v1+html; q=0.1, text/html; q=0.01";
const UV: &str = "application/vnd.pypi.simple.v1+json, application/vnd.pypi.simple.v1+html;q=0.2, text/html;q=0.01";
const POETRY: &str = "application/vnd.pypi.simple.v1+json, */*;q=0.1";

#[test]
fn json_is_chosen_only_when_the_client_names_it() {
    // Arrange
    let headers = [
        Some(PIP),
        Some(UV),
        Some(POETRY),
        Some("application/vnd.pypi.simple.latest+json"),
        Some("Application/Vnd.Pypi.Simple.V1+JSON"),
        Some("*/*"),
        Some("text/html"),
        Some("application/vnd.pypi.simple.v1+html"),
        None,
    ];

    // Act
    let actual = headers.map(View::from_accept);

    // Assert
    assert_eq!(
        actual,
        [
            View::Json,
            View::Json,
            View::Json,
            View::Json,
            View::Json,
            View::Html,
            View::Html,
            View::Html,
            View::Html,
        ]
    );
}

#[test]
fn content_types_are_exactly_what_pip_and_poetry_check() {
    // Arrange
    let views = [View::Json, View::Html];

    // Act
    let actual = views.map(content_type);

    // Assert
    assert_eq!(
        actual,
        [
            "application/vnd.pypi.simple.v1+json",
            "text/html; charset=utf-8"
        ]
    );
}
