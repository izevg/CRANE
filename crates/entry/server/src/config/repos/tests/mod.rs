#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use crate::config::tests::{error_of, parse, with};

#[test]
fn proxy_repos_list_is_parsed_and_empty_values_mean_no_proxy() {
    // Arrange
    let cases = [
        with(&[]),
        with(&[(CARGO_PROXY_REPOS, "")]),
        with(&[(CARGO_PROXY_REPOS, "crates-io")]),
        with(&[(CARGO_PROXY_REPOS, " crates-io , mirror.2 ")]),
    ];

    // Act
    let actual = cases.map(|pairs| {
        parse(&pairs)
            .unwrap()
            .repos
            .names(Format::Cargo, RepositoryKind::Proxy)
            .to_vec()
    });

    // Assert
    assert_eq!(
        actual,
        [
            Vec::<String>::new(),
            Vec::new(),
            vec!["crates-io".to_owned()],
            vec!["crates-io".to_owned(), "mirror.2".to_owned()],
        ]
    );
}

#[test]
fn invalid_proxy_repos_are_rejected_with_reason() {
    // Arrange
    let cases = [
        with(&[(CARGO_PROXY_REPOS, "crates-io,,other")]),
        with(&[(CARGO_PROXY_REPOS, "internal")]),
        with(&[(CARGO_PROXY_REPOS, "a,a")]),
        with(&[(CARGO_PROXY_REPOS, "a{b}")]),
        with(&[(CARGO_PROXY_REPOS, "a/b")]),
    ];

    // Act
    let actual = cases.map(|pairs| error_of(&pairs));

    // Assert
    let invalid = |reason| ConfigError::Invalid {
        var: CARGO_PROXY_REPOS,
        reason,
    };
    assert_eq!(
        actual,
        [
            invalid("пустое имя репозитория в списке"),
            invalid("имя совпадает с hosted-репозиторием"),
            invalid("имя повторяется в списке"),
            invalid("имя может содержать только буквы, цифры, `-`, `_` и `.`"),
            invalid("имя может содержать только буквы, цифры, `-`, `_` и `.`"),
        ]
    );
}

#[test]
fn npm_repos_default_to_none_and_are_parsed_per_variable() {
    // Arrange
    let cases = [
        with(&[]),
        with(&[(NPM_REPOS, ""), (NPM_PROXY_REPOS, "")]),
        with(&[(NPM_REPOS, " web , libs.2 "), (NPM_PROXY_REPOS, "npmjs")]),
    ];

    // Act
    let actual = cases.map(|pairs| {
        let settings = parse(&pairs).unwrap();
        (
            settings
                .repos
                .names(Format::Npm, RepositoryKind::Hosted)
                .to_vec(),
            settings
                .repos
                .names(Format::Npm, RepositoryKind::Proxy)
                .to_vec(),
            settings
                .repos
                .names(Format::Cargo, RepositoryKind::Hosted)
                .to_vec(),
        )
    });

    // Assert
    let internal = vec!["internal".to_owned()];
    assert_eq!(
        actual,
        [
            (vec![], vec![], internal.clone()),
            (vec![], vec![], internal.clone()),
            (
                vec!["web".to_owned(), "libs.2".to_owned()],
                vec!["npmjs".to_owned()],
                internal,
            ),
        ]
    );
}

#[test]
fn invalid_npm_repos_are_rejected_with_variable_and_reason() {
    // Arrange
    let cases = [
        with(&[(NPM_REPOS, "web,,libs")]),
        with(&[(NPM_REPOS, "a/b")]),
        with(&[(NPM_REPOS, "web,web")]),
        with(&[(NPM_PROXY_REPOS, "web"), (NPM_REPOS, "web")]),
        with(&[(NPM_PROXY_REPOS, "a{b}")]),
        with(&[(NPM_PROXY_REPOS, "p,p")]),
    ];

    // Act
    let actual = cases.map(|pairs| error_of(&pairs));

    // Assert
    let invalid = |var, reason| ConfigError::Invalid { var, reason };
    let charset = "имя может содержать только буквы, цифры, `-`, `_` и `.`";
    assert_eq!(
        actual,
        [
            invalid(NPM_REPOS, "пустое имя репозитория в списке"),
            invalid(NPM_REPOS, charset),
            invalid(NPM_REPOS, "имя повторяется в списке"),
            invalid(NPM_PROXY_REPOS, "имя совпадает с hosted-репозиторием"),
            invalid(NPM_PROXY_REPOS, charset),
            invalid(NPM_PROXY_REPOS, "имя повторяется в списке"),
        ]
    );
}

#[test]
fn same_name_may_be_hosted_in_one_format_and_proxy_in_another() {
    // Arrange
    let pairs = with(&[(CARGO_REPOS, "shared"), (NPM_PROXY_REPOS, "shared")]);

    // Act
    let settings = parse(&pairs).unwrap();

    // Assert
    assert_eq!(
        (
            settings.repos.names(Format::Cargo, RepositoryKind::Hosted),
            settings.repos.names(Format::Npm, RepositoryKind::Proxy)
        ),
        (&["shared".to_owned()][..], &["shared".to_owned()][..])
    );
}

#[test]
fn repos_list_is_split_and_trimmed() {
    // Arrange
    let pairs = with(&[(CARGO_REPOS, " alpha , beta,gamma ")]);

    // Act
    let settings = parse(&pairs).unwrap();

    // Assert
    assert_eq!(
        settings.repos.names(Format::Cargo, RepositoryKind::Hosted),
        ["alpha", "beta", "gamma"]
    );
}

#[test]
fn repos_list_rejects_empty_items() {
    // Arrange
    let trailing = with(&[(CARGO_REPOS, "alpha,")]);
    let blank = with(&[(CARGO_REPOS, "alpha, ,beta")]);

    // Act
    let errors = [trailing, blank].map(|pairs| error_of(&pairs));

    // Assert
    let empty = || ConfigError::Invalid {
        var: CARGO_REPOS,
        reason: "пустое имя репозитория в списке",
    };
    assert_eq!(errors, [empty(), empty()]);
}

#[test]
fn zero_publish_limit_is_rejected() {
    // Arrange
    let pairs = with(&[(MAX_PUBLISH_BYTES, "0")]);

    // Act
    let error = error_of(&pairs);

    // Assert
    assert!(matches!(
        error,
        ConfigError::Invalid {
            var: MAX_PUBLISH_BYTES,
            ..
        }
    ));
}
