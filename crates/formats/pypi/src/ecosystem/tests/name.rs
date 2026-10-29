use crane_domain::registry::NameError;

use crate::ecosystem::parse_name;

#[test]
fn valid_names_keep_their_spelling_and_normalize_per_pep_503() {
    // Arrange
    let raws = [
        "Requests",
        "zope.interface",
        "My_Pkg-Name",
        "a--b__c..d",
        "A1",
    ];

    // Act
    let actual = raws.map(|raw| {
        let name = parse_name(raw).unwrap();
        (name.as_written().to_owned(), name.normalized().to_owned())
    });

    // Assert
    let expected = [
        ("Requests", "requests"),
        ("zope.interface", "zope-interface"),
        ("My_Pkg-Name", "my-pkg-name"),
        ("a--b__c..d", "a-b-c-d"),
        ("A1", "a1"),
    ]
    .map(|(written, normalized)| (written.to_owned(), normalized.to_owned()));
    assert_eq!(actual, expected);
}

#[test]
fn spellings_that_differ_only_in_case_and_separator_runs_are_one_name() {
    // Arrange
    let raws = ["Foo_Bar", "foo.bar", "FOO---bar", "foo-_.bar"];

    // Act
    let actual = raws.map(|raw| parse_name(raw).unwrap().normalized().to_owned());

    // Assert
    assert_eq!(actual, ["foo-bar"; 4].map(str::to_owned));
}

#[test]
fn single_alphanumeric_character_is_a_valid_name() {
    // Arrange
    let raw = "a";

    // Act
    let result = parse_name(raw);

    // Assert
    assert_eq!(result.unwrap().normalized(), "a");
}

#[test]
fn names_outside_the_pep_508_grammar_are_rejected_with_the_reason() {
    // Arrange
    let raws = ["", "-a", "_a", ".a", "a-", "a.", "a b", "ünï", "a/b", "a@b"];

    // Act
    let actual = raws.map(|raw| parse_name(raw).unwrap_err());

    // Assert
    assert_eq!(
        actual,
        [
            NameError::Empty,
            NameError::InvalidStart('-'),
            NameError::InvalidStart('_'),
            NameError::InvalidStart('.'),
            NameError::InvalidChar('-'),
            NameError::InvalidChar('.'),
            NameError::InvalidChar(' '),
            NameError::InvalidChar('ü'),
            NameError::InvalidChar('/'),
            NameError::InvalidChar('@'),
        ]
    );
}

#[test]
fn name_is_limited_to_255_bytes() {
    // Arrange
    let (at_limit, over_limit) = ("a".repeat(255), "a".repeat(256));

    // Act
    let actual = (
        parse_name(&at_limit).is_ok(),
        parse_name(&over_limit).unwrap_err(),
    );

    // Assert
    assert_eq!(actual, (true, NameError::TooLong { len: 256, max: 255 }));
}
