use axum::http::{HeaderMap, HeaderName, HeaderValue};

use super::*;

type Pairs<'a> = Vec<(&'a str, &'a [u8])>;

const AUTHORIZATION: &str = "authorization";
const NUGET_KEY: &str = "x-nuget-apikey";

fn headers(pairs: &[(&str, &[u8])]) -> HeaderMap {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        map.append(
            HeaderName::from_bytes(name.as_bytes()).unwrap(),
            HeaderValue::from_bytes(value).unwrap(),
        );
    }
    map
}

fn basic(credentials: &str) -> Vec<u8> {
    let mut value = b"Basic ".to_vec();
    value.extend(STANDARD.encode(credentials).into_bytes());
    value
}

fn secret(format: Format, pairs: &[(&str, &[u8])]) -> Option<Vec<u8>> {
    presented(format, &headers(pairs)).map(Cow::into_owned)
}

#[test]
fn raw_and_bearer_read_authorization_by_their_own_rules() {
    // Arrange
    let cases: [(Format, &[u8]); 8] = [
        (Format::Cargo, b"tok"),
        (Format::Cargo, b"Bearer tok"),
        (Format::Npm, b"Bearer tok"),
        (Format::Npm, b"bearer tok"),
        (Format::Npm, b"BEARER tok"),
        (Format::Npm, b"tok"),
        (Format::Npm, b"Basic tok"),
        (Format::Npm, b"Bearer"),
    ];

    // Act
    let actual = cases.map(|(format, value)| secret(format, &[(AUTHORIZATION, value)]));

    // Assert
    let expected: [Option<&[u8]>; 8] = [
        Some(b"tok"),
        Some(b"Bearer tok"),
        Some(b"tok"),
        Some(b"tok"),
        Some(b"tok"),
        None,
        None,
        None,
    ];
    assert_eq!(actual, expected.map(|value| value.map(<[u8]>::to_vec)));
}

#[test]
fn basic_takes_the_password_after_the_first_colon() {
    // Arrange
    let cases = [
        basic("user:s3cret"),
        basic("user:a:b"),
        basic(":s3cret"),
        basic("user:"),
        basic("user-without-colon"),
        b"Basic !!not-base64!!".to_vec(),
        b"BASIC dXNlcjpzM2NyZXQ=".to_vec(),
        b"basic dXNlcjpzM2NyZXQ=".to_vec(),
    ];

    // Act
    let actual = cases.map(|value| secret(Format::PyPi, &[(AUTHORIZATION, &value)]));

    // Assert
    let some = |text: &str| Some(text.as_bytes().to_vec());
    assert_eq!(
        actual,
        [
            some("s3cret"),
            some("a:b"),
            some("s3cret"),
            some(""),
            None,
            None,
            some("s3cret"),
            some("s3cret"),
        ]
    );
}

#[test]
fn basic_formats_fall_back_to_bearer_and_reject_foreign_schemes() {
    // Arrange
    let cases: [(Format, &[u8]); 4] = [
        (Format::PyPi, b"Bearer tok"),
        (Format::Maven, b"Bearer tok"),
        (Format::NuGet, b"Bearer tok"),
        (Format::PyPi, b"Digest abc"),
    ];

    // Act
    let actual = cases.map(|(format, value)| secret(format, &[(AUTHORIZATION, value)]));

    // Assert
    let tok = Some(b"tok".to_vec());
    assert_eq!(actual, [tok.clone(), tok, None, None]);
}

#[test]
fn nuget_prefers_the_api_key_header_over_basic() {
    // Arrange
    let password = basic("user:from-basic");
    let cases: [Pairs<'_>; 5] = [
        vec![(NUGET_KEY, b"from-key")],
        vec![(AUTHORIZATION, &password)],
        vec![(AUTHORIZATION, &password), (NUGET_KEY, b"from-key")],
        vec![(NUGET_KEY, b"")],
        vec![],
    ];

    // Act
    let actual = cases.map(|pairs| secret(Format::NuGet, &pairs));

    // Assert
    let some = |text: &str| Some(text.as_bytes().to_vec());
    assert_eq!(
        actual,
        [
            some("from-key"),
            some("from-basic"),
            some("from-key"),
            some(""),
            None
        ]
    );
}

#[test]
fn api_key_header_is_ignored_by_formats_that_do_not_accept_it() {
    // Arrange
    let formats = [Format::Cargo, Format::Npm, Format::PyPi, Format::Maven];

    // Act
    let actual = formats.map(|format| secret(format, &[(NUGET_KEY, b"key")]));

    // Assert
    assert_eq!(actual, [None, None, None, None]);
}

#[test]
fn offered_means_the_client_sent_a_header_the_format_accepts() {
    // Arrange
    let cases: [(Format, Pairs<'_>); 6] = [
        (Format::Npm, vec![]),
        (Format::Npm, vec![(AUTHORIZATION, b"Basic x")]),
        (Format::Npm, vec![(NUGET_KEY, b"key")]),
        (Format::NuGet, vec![(NUGET_KEY, b"key")]),
        (Format::NuGet, vec![(AUTHORIZATION, b"Bearer x")]),
        (Format::PyPi, vec![]),
    ];

    // Act
    let actual = cases.map(|(format, pairs)| offered(format, &headers(&pairs)));

    // Assert
    assert_eq!(actual, [false, true, false, true, true, false]);
}
