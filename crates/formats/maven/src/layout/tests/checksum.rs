use crane_domain::registry::{DigestError, Digests, Md5, Sha1, Sha256, Sha512};

use crate::layout::Algorithm;

const MD5_OF_ABC: &str = "900150983cd24fb0d6963f7d28e17f72";
const SHA1_OF_ABC: &str = "a9993e364706816aba3e25717850c26c9cd0d89d";
const SHA256_OF_ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
const SHA512_OF_ABC: &str = "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f";

#[test]
fn document_hash_is_lowercase_hex_of_the_algorithm() {
    // Arrange
    let bytes = b"abc";

    // Act
    let actual = Algorithm::ALL.map(|algorithm| algorithm.hash(bytes));

    // Assert
    assert_eq!(
        actual,
        [
            MD5_OF_ABC.to_owned(),
            SHA1_OF_ABC.to_owned(),
            SHA256_OF_ABC.to_owned(),
            SHA512_OF_ABC.to_owned(),
        ]
    );
}

#[test]
fn declared_sum_is_the_first_token_in_lowercase() {
    // Arrange
    let bodies = [
        "A9993E364706816ABA3E25717850C26C9CD0D89D",
        "a9993e364706816aba3e25717850c26c9cd0d89d\n",
        "a9993e364706816aba3e25717850c26c9cd0d89d  core-1.0.jar\n",
        "  \t a9993e364706816aba3e25717850c26c9cd0d89d",
    ];

    // Act
    let actual = bodies.map(|body| Algorithm::Sha1.parse_declared(body));

    // Assert
    assert_eq!(actual, bodies.map(|_| Ok(SHA1_OF_ABC.to_owned())));
}

#[test]
fn declared_sum_must_have_the_length_of_its_algorithm() {
    // Arrange
    let zeros = |len: usize| "0".repeat(len);
    let cases = [
        (Algorithm::Md5, zeros(32), Ok(zeros(32))),
        (Algorithm::Sha1, zeros(40), Ok(zeros(40))),
        (Algorithm::Sha256, zeros(64), Ok(zeros(64))),
        (Algorithm::Sha512, zeros(128), Ok(zeros(128))),
        (
            Algorithm::Md5,
            zeros(40),
            Err(DigestError::InvalidLength {
                expected: 32,
                actual: 40,
            }),
        ),
        (
            Algorithm::Sha1,
            zeros(39),
            Err(DigestError::InvalidLength {
                expected: 40,
                actual: 39,
            }),
        ),
        (
            Algorithm::Sha256,
            zeros(65),
            Err(DigestError::InvalidLength {
                expected: 64,
                actual: 65,
            }),
        ),
        (
            Algorithm::Sha512,
            zeros(64),
            Err(DigestError::InvalidLength {
                expected: 128,
                actual: 64,
            }),
        ),
        (
            Algorithm::Sha1,
            String::new(),
            Err(DigestError::InvalidLength {
                expected: 40,
                actual: 0,
            }),
        ),
        (
            Algorithm::Sha1,
            " \n".to_owned(),
            Err(DigestError::InvalidLength {
                expected: 40,
                actual: 0,
            }),
        ),
    ];

    // Act
    let actual = cases
        .each_ref()
        .map(|(algorithm, body, _)| algorithm.parse_declared(body));

    // Assert
    assert_eq!(actual, cases.map(|(_, _, expected)| expected));
}

#[test]
fn declared_sum_with_a_non_hex_character_is_rejected() {
    // Arrange
    let body = format!("g{}", "0".repeat(39));

    // Act
    let result = Algorithm::Sha1.parse_declared(&body);

    // Assert
    assert_eq!(result, Err(DigestError::InvalidChar('g')));
}

#[test]
fn file_sum_is_taken_from_catalog_digests() {
    // Arrange
    let digests = Digests {
        sha256: Sha256::from([1; 32]),
        sha512: Sha512::from([2; 64]),
        sha1: Sha1::from([3; 20]),
        md5: Md5::from([4; 16]),
        size: 0,
    };

    // Act
    let actual = Algorithm::ALL.map(|algorithm| algorithm.of(&digests));

    // Assert
    assert_eq!(
        actual,
        [
            "04".repeat(16),
            "03".repeat(20),
            "01".repeat(32),
            "02".repeat(64),
        ]
    );
}
