use std::collections::hash_map::DefaultHasher;

use super::*;

fn name(normalized: &str, written: &str) -> PackageName {
    PackageName::from_parts(normalized.to_owned(), written.to_owned())
}

fn hash_of(name: &PackageName) -> u64 {
    let mut hasher = DefaultHasher::new();
    name.hash(&mut hasher);
    hasher.finish()
}

#[test]
fn names_with_equal_normalized_form_are_equal_and_hash_alike() {
    // Arrange
    let (a, b) = (name("foo_bar", "Foo-Bar"), name("foo_bar", "FOO_BAR"));

    // Act
    let actual = (a == b, hash_of(&a) == hash_of(&b));

    // Assert
    assert_eq!(actual, (true, true));
}

#[test]
fn names_with_different_normalized_form_are_not_equal() {
    // Arrange
    let (a, b) = (name("foo_bar", "foo-bar"), name("foo_baz", "foo-baz"));

    // Act
    let equal = a == b;

    // Assert
    assert!(!equal);
}

#[test]
fn written_form_is_preserved_as_is() {
    // Arrange
    let name = name("foo_bar", "Foo-Bar");

    // Act
    let (written, shown) = (name.as_written().to_owned(), name.to_string());

    // Assert
    assert_eq!((written.as_str(), shown.as_str()), ("Foo-Bar", "Foo-Bar"));
}
