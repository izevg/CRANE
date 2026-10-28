use axum::http::HeaderValue;

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Create,
    Update(u64),
    /// Статус отказа.
    Refused(u16),
}

fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        map.append(*name, HeaderValue::from_static(value));
    }
    map
}

fn put(pairs: &[(&'static str, &'static str)]) -> Outcome {
    match for_put(&headers(pairs)) {
        Ok(None) => Outcome::Create,
        Ok(Some(version)) => Outcome::Update(version.get()),
        Err(problem) => Outcome::Refused(refusal(problem)),
    }
}

fn refusal(problem: Problem) -> u16 {
    use axum::response::IntoResponse;
    problem.into_response().status().as_u16()
}

#[test]
fn put_creates_with_if_none_match_star_and_updates_with_a_quoted_version() {
    // Arrange
    let cases: [&[(&str, &str)]; 2] = [&[("if-none-match", "*")], &[("if-match", "\"12\"")]];

    // Act
    let actual = cases.map(put);

    // Assert
    assert_eq!(actual, [Outcome::Create, Outcome::Update(12)]);
}

#[test]
fn put_without_any_condition_requires_one() {
    // Arrange
    let none: &[(&str, &str)] = &[];

    // Act
    let actual = put(none);

    // Assert
    assert_eq!(actual, Outcome::Refused(428));
}

#[test]
fn put_rejects_malformed_or_ambiguous_conditions_as_bad_request() {
    // Arrange
    let cases: [&[(&str, &str)]; 8] = [
        &[("if-match", "12")],
        &[("if-match", "W/\"12\"")],
        &[("if-match", "\"0\"")],
        &[("if-match", "\"abc\"")],
        &[("if-match", "*")],
        &[("if-none-match", "\"1\"")],
        &[("if-match", "\"1\""), ("if-none-match", "*")],
        &[("if-match", "\"1\""), ("if-match", "\"2\"")],
    ];

    // Act
    let actual = cases.map(put);

    // Assert
    assert_eq!(actual.to_vec(), vec![Outcome::Refused(400); 8]);
}

#[test]
fn delete_needs_exactly_one_quoted_version() {
    // Arrange
    let cases: [&[(&str, &str)]; 3] = [&[("if-match", "\"3\"")], &[], &[("if-match", "3")]];

    // Act
    let actual = cases.map(|pairs| match for_delete(&headers(pairs)) {
        Ok(version) => Outcome::Update(version.get()),
        Err(problem) => Outcome::Refused(refusal(problem)),
    });

    // Assert
    assert_eq!(
        actual,
        [
            Outcome::Update(3),
            Outcome::Refused(428),
            Outcome::Refused(400)
        ]
    );
}
