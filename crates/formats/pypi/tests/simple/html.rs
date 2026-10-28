use crane_format_pypi::{View, Yanked, render};

use super::{dist, project};

const WHEEL: &str = "acme_core-1.0-py3-none-any.whl";

fn anchor_of(html: &str) -> &str {
    html.lines().find(|line| line.starts_with("<a ")).unwrap()
}

#[test]
fn page_is_pep_503_html5_with_repository_version_and_hash_fragment() {
    // Arrange
    let project = project(vec![dist(WHEEL)]);

    // Act
    let html = render(&project, View::Html);

    // Assert
    let hash = "ab".repeat(32);
    assert_eq!(
        html,
        format!(
            "<!DOCTYPE html>\n<html>\n<head>\n\
             <meta name=\"pypi:repository-version\" content=\"1.1\">\n\
             <title>Links for acme-core</title>\n</head>\n<body>\n\
             <h1>Links for acme-core</h1>\n\
             <a href=\"https://crane.example/pypi/internal/files/acme/1/{WHEEL}#sha256={hash}\">{WHEEL}</a><br/>\n\
             </body>\n</html>\n"
        )
    );
}

#[test]
fn requires_python_is_an_escaped_attribute() {
    // Arrange
    let mut file = dist(WHEEL);
    file.requires_python = Some(">=3.8,<4".to_owned());

    // Act
    let html = render(&project(vec![file]), View::Html);

    // Assert
    assert!(
        anchor_of(&html).contains(" data-requires-python=\"&gt;=3.8,&lt;4\">"),
        "{html}"
    );
}

#[test]
fn yanked_without_reason_is_a_bare_attribute() {
    // Arrange
    let mut file = dist(WHEEL);
    file.yanked = Yanked::Yes(None);

    // Act
    let html = render(&project(vec![file]), View::Html);

    // Assert
    assert!(anchor_of(&html).contains(" data-yanked>"), "{html}");
}

#[test]
fn yanked_reason_is_an_escaped_attribute_value() {
    // Arrange
    let mut file = dist(WHEEL);
    file.yanked = Yanked::Yes(Some("bad \"x\" & <y>".to_owned()));

    // Act
    let html = render(&project(vec![file]), View::Html);

    // Assert
    assert!(
        anchor_of(&html).contains(" data-yanked=\"bad &quot;x&quot; &amp; &lt;y&gt;\">"),
        "{html}"
    );
}

#[test]
fn live_file_has_no_yanked_or_requires_python_attributes() {
    // Arrange
    let file = dist(WHEEL);

    // Act
    let html = render(&project(vec![file]), View::Html);

    // Assert
    assert!(!anchor_of(&html).contains("data-"), "{}", anchor_of(&html));
}

#[test]
fn markup_in_names_and_links_cannot_break_out_of_the_page() {
    // Arrange
    let mut file = dist("a<b>&\"c.whl");
    file.url = "https://h/x?a=1&b=\"2\"".to_owned();

    // Act
    let html = render(&project(vec![file]), View::Html);

    // Assert
    assert!(
        anchor_of(&html).starts_with("<a href=\"https://h/x?a=1&amp;b=&quot;2&quot;#sha256=")
            && anchor_of(&html).ends_with(">a&lt;b&gt;&amp;&quot;c.whl</a><br/>"),
        "{html}"
    );
}
