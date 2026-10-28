use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crane_domain::registry::{NameError, PackageName};

use crate::ecosystem::parse_name;

fn cargo(raw: &str) -> Result<PackageName, NameError> {
    parse_name(raw)
}

fn hash_of(name: &PackageName) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn dash_underscore_and_case_variants_are_equal() {
    // Arrange
    let a = cargo("Foo-Bar").unwrap();
    let b = cargo("foo_bar").unwrap();
    let c = cargo("FOO_BAR").unwrap();

    // Act
    let equal = a == b && b == c;

    // Assert
    assert!(equal);
}

#[test]
fn equal_names_have_equal_hash() {
    // Arrange
    let a = cargo("Foo-Bar").unwrap();
    let b = cargo("FOO_BAR").unwrap();

    // Act
    let (ha, hb) = (hash_of(&a), hash_of(&b));

    // Assert
    assert_eq!(ha, hb);
}

#[test]
fn different_names_are_not_equal() {
    // Arrange
    let a = cargo("foo-bar").unwrap();
    let b = cargo("foo-baz").unwrap();

    // Act
    let equal = a == b;

    // Assert
    assert!(!equal);
}

#[test]
fn normalized_form_is_lowercase_with_underscores() {
    // Arrange
    let name = cargo("Foo-Bar").unwrap();

    // Act
    let normalized = name.normalized();

    // Assert
    assert_eq!(normalized, "foo_bar");
}

#[test]
fn written_form_is_preserved_as_is() {
    // Arrange
    let name = cargo("Foo-Bar").unwrap();

    // Act
    let (written, shown) = (name.as_written().to_owned(), name.to_string());

    // Assert
    assert_eq!((written.as_str(), shown.as_str()), ("Foo-Bar", "Foo-Bar"));
}

#[test]
fn name_of_max_length_is_accepted() {
    // Arrange
    let raw = "a".repeat(64);

    // Act
    let result = cargo(&raw);

    // Assert
    assert!(result.is_ok());
}

#[test]
fn name_over_max_length_is_rejected() {
    // Arrange
    let raw = "a".repeat(65);

    // Act
    let result = cargo(&raw);

    // Assert
    assert_eq!(result.unwrap_err(), NameError::TooLong { len: 65, max: 64 });
}

#[test]
fn empty_name_is_rejected() {
    // Arrange
    let raw = "";

    // Act
    let result = cargo(raw);

    // Assert
    assert_eq!(result.unwrap_err(), NameError::Empty);
}

#[test]
fn name_starting_with_non_letter_is_rejected() {
    // Arrange
    let raws = ["1abc", "-abc", "_abc"];

    // Act
    let errors: Vec<_> = raws.iter().map(|r| cargo(r).unwrap_err()).collect();

    // Assert
    assert_eq!(
        errors,
        [
            NameError::InvalidStart('1'),
            NameError::InvalidStart('-'),
            NameError::InvalidStart('_'),
        ]
    );
}

#[test]
fn name_with_forbidden_character_is_rejected() {
    // Arrange
    let raws = ["abé", "a b", "a.b"];

    // Act
    let errors: Vec<_> = raws.iter().map(|r| cargo(r).unwrap_err()).collect();

    // Assert
    assert_eq!(
        errors,
        [
            NameError::InvalidChar('é'),
            NameError::InvalidChar(' '),
            NameError::InvalidChar('.'),
        ]
    );
}
