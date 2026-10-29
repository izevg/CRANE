use crane_domain::registry::{Checksum, Md5, Sha256};
use crane_format_pypi::admit;
use serde_json::{Value, json};

use super::{SDIST, WHEEL, sha256_hex, wheel_form, with, without};

#[test]
fn wheel_upload_is_accepted_with_names_version_digest_and_metadata() {
    // Arrange
    let fields = with(wheel_form(), "sha256_digest", &sha256_hex());
    let fields = with(fields, "requires_python", ">=3.8");
    let fields = with(fields, "license_expression", "MIT");

    // Act
    let accepted = admit(&fields, WHEEL).unwrap();

    // Assert
    let metadata: Value = serde_json::from_str(accepted.metadata.as_str()).unwrap();
    assert_eq!(
        (
            accepted.package.normalized().to_owned(),
            accepted.package.as_written().to_owned(),
            accepted.version.canonical().to_owned(),
            accepted.filename,
            accepted.declared,
            metadata,
        ),
        (
            "acme-core".to_owned(),
            "Acme_Core".to_owned(),
            "1".to_owned(),
            WHEEL.to_owned(),
            vec![Checksum::Sha256(Sha256::from_hex(&sha256_hex()).unwrap())],
            json!({"requires_python": ">=3.8", "license_expression": "MIT"}),
        )
    );
}

#[test]
fn sdist_upload_is_accepted() {
    // Arrange
    let fields = with(wheel_form(), "filetype", "sdist");

    // Act
    let accepted = admit(&fields, SDIST).unwrap();

    // Assert
    assert_eq!(accepted.filename, SDIST);
}

#[test]
fn project_and_version_are_compared_after_normalization() {
    // Arrange
    let cases = [
        ("acme.core", "1.0", "Acme_Core-1.0.0-py3-none-any.whl"),
        ("ACME-CORE", "1.0.0", "acme_core-1-py3-none-any.whl"),
        (
            "acme_core",
            "v1.0.0-rc1",
            "acme_core-1.0.0rc1-py3-none-any.whl",
        ),
    ];

    // Act
    let actual = cases.map(|(name, version, file)| {
        let fields = with(with(wheel_form(), "name", name), "version", version);
        admit(&fields, file).is_ok()
    });

    // Assert
    assert_eq!(actual, [true; 3]);
}

#[test]
fn wheel_with_a_build_tag_is_accepted() {
    // Arrange
    let fields = wheel_form();

    // Act
    let result = admit(&fields, "acme_core-1.0.0-1-py3-none-any.whl");

    // Assert
    assert!(result.is_ok());
}

#[test]
fn legacy_sdist_with_a_dash_in_the_project_name_is_split_at_the_last_valid_version() {
    // Arrange
    let fields = with(with(wheel_form(), "filetype", "sdist"), "name", "my-pkg");

    // Act
    let result = admit(&fields, "my-pkg-1.0.0.tar.gz");

    // Assert
    assert!(result.is_ok());
}

#[test]
fn absent_empty_and_blake2_digests_declare_nothing() {
    // Arrange
    let blake = with(wheel_form(), "blake2_256_digest", "zz");
    let empty_sha = with(wheel_form(), "sha256_digest", "");
    let empty_md5 = with(wheel_form(), "md5_digest", "");
    let cases = [wheel_form(), blake, empty_sha, empty_md5];

    // Act
    let actual = cases.map(|fields| admit(&fields, WHEEL).unwrap().declared);

    // Assert
    assert_eq!(actual, [vec![], vec![], vec![], vec![]]);
}

#[test]
fn md5_digest_is_decoded_from_urlsafe_base64_next_to_sha256() {
    // Arrange
    let fields = with(wheel_form(), "sha256_digest", &sha256_hex());
    let cases = [
        "1B2M2Y8AsgTpgAmY7PhCfg",
        "1B2M2Y8AsgTpgAmY7PhCfg==",
        "-_-_-_-_-_-_-_-_-_-_-w",
    ];

    // Act
    let actual = cases.map(|md5| {
        let fields = with(fields.clone(), "md5_digest", md5);
        admit(&fields, WHEEL).unwrap().declared[1]
    });

    // Assert
    let md5 = |hex| Checksum::Md5(Md5::from_hex(hex).unwrap());
    assert_eq!(
        actual,
        [
            md5("d41d8cd98f00b204e9800998ecf8427e"),
            md5("d41d8cd98f00b204e9800998ecf8427e"),
            md5("fbffbffbffbffbffbffbffbffbffbffb"),
        ]
    );
}

#[test]
fn uv_style_empty_requires_python_is_stored_as_unknown() {
    // Arrange
    let fields = with(wheel_form(), "requires_python", "");

    // Act
    let accepted = admit(&fields, WHEEL).unwrap();

    // Assert
    let metadata: Value = serde_json::from_str(accepted.metadata.as_str()).unwrap();
    assert_eq!(metadata, json!({}));
}

#[test]
fn repeated_multi_value_fields_do_not_disturb_admission() {
    // Arrange
    let mut fields = wheel_form();
    fields.extend(
        [
            ("classifiers", "A"),
            ("classifiers", "B"),
            ("requires_dist", "x"),
        ]
        .map(|(k, v)| (k.to_owned(), v.to_owned())),
    );

    // Act
    let result = admit(&fields, WHEEL);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn uppercase_sha256_hex_is_accepted_as_the_same_digest() {
    // Arrange
    let fields = with(wheel_form(), "sha256_digest", &sha256_hex().to_uppercase());

    // Act
    let accepted = admit(&fields, WHEEL).unwrap();

    // Assert
    assert_eq!(
        accepted.declared,
        vec![Checksum::Sha256(Sha256::from([0xab; 32]))]
    );
}

#[test]
fn form_without_a_digest_is_accepted_and_the_blob_store_hashes_the_body() {
    // Arrange
    let fields = without(wheel_form(), "sha256_digest");

    // Act
    let accepted = admit(&fields, WHEEL).unwrap();

    // Assert
    assert_eq!(accepted.declared, Vec::<Checksum>::new());
}
