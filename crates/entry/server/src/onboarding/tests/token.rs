#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap - это падение теста"
)]

use super::*;

#[test]
fn base64url_matches_rfc_4648_vectors_in_the_url_alphabet_without_padding() {
    // Arrange
    let cases: [&[u8]; 6] = [b"", b"f", b"fo", b"foo", b"foob", &[0xfb, 0xff]];

    // Act
    let encoded = cases.map(base64url);

    // Assert
    assert_eq!(encoded, ["", "Zg", "Zm8", "Zm9v", "Zm9vYg", "-_8"]);
}

#[test]
fn generated_token_is_prefix_and_43_url_safe_symbols_of_256_bits() {
    // Arrange
    let generate = BootstrapToken::generate;

    // Act
    let tokens = [generate().unwrap(), generate().unwrap()];

    // Assert
    let texts = tokens.map(|token| token.expose().to_owned());
    let well_formed = texts.clone().map(|text| {
        text.strip_prefix("crb_").is_some_and(|body| {
            body.len() == 43
                && body
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        })
    });
    assert_eq!((well_formed, texts[0] == texts[1]), ([true, true], false));
}

#[test]
fn hash_is_taken_over_the_whole_token_text() {
    // Arrange
    let token = BootstrapToken::provided(Secret::new("crb_known".to_owned()));

    // Act
    let hash = token.hash();

    // Assert
    assert!(hash.matches(b"crb_known"));
}
