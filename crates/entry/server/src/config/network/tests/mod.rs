#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

use std::net::IpAddr;

use crate::config::tests::{env, error_of, parse, with};

#[test]
fn invalid_ports_are_rejected_naming_the_variable() {
    // Arrange
    let too_big = with(&[(HTTP_PORT, "65536")]);
    let text = with(&[(ADMIN_PORT, "abc")]);

    // Act
    let errors = [too_big, text].map(|pairs| error_of(&pairs));

    // Assert
    let reason = "ожидалось целое число в допустимом диапазоне";
    assert_eq!(
        errors,
        [
            ConfigError::Invalid {
                var: HTTP_PORT,
                reason
            },
            ConfigError::Invalid {
                var: ADMIN_PORT,
                reason
            }
        ]
    );
}

#[test]
fn public_url_requires_http_scheme_and_host() {
    // Arrange
    let no_scheme = with(&[(PUBLIC_URL, "crane.example")]);
    let ftp = with(&[(PUBLIC_URL, "ftp://crane.example")]);
    let no_host = with(&[(PUBLIC_URL, "https:///")]);

    // Act
    let errors = [no_scheme, ftp, no_host].map(|pairs| error_of(&pairs));

    // Assert
    let invalid = |reason| ConfigError::Invalid {
        var: PUBLIC_URL,
        reason,
    };
    let scheme = "адрес должен начинаться с http:// или https://";
    assert_eq!(
        errors,
        [
            invalid(scheme),
            invalid(scheme),
            invalid("в адресе нет хоста")
        ]
    );
}

#[test]
fn public_url_accepts_http_and_trims_trailing_slashes() {
    // Arrange
    let pairs = with(&[(PUBLIC_URL, "http://127.0.0.1:8080//")]);

    // Act
    let settings = parse(&pairs).unwrap();

    // Assert
    assert_eq!(settings.public_url, "http://127.0.0.1:8080");
}

#[test]
fn trusted_proxies_default_to_none_and_follow_the_list() {
    // Arrange
    let cases = [with(&[]), with(&[(TRUSTED_PROXIES, "10.0.0.0/8, ::1")])];
    let peers: [IpAddr; 3] = ["10.1.2.3", "::1", "203.0.113.1"].map(|p| p.parse().unwrap());

    // Act
    let trusted = cases.map(|pairs| {
        let settings = parse(&pairs).unwrap();
        peers.map(|peer| settings.trusted_proxies.contains(peer))
    });

    // Assert
    assert_eq!(trusted, [[false; 3], [true, true, false]]);
}

#[test]
fn malformed_trusted_proxies_are_rejected() {
    // Arrange
    let pairs = with(&[(TRUSTED_PROXIES, "10.0.0.0/33")]);

    // Act
    let error = error_of(&pairs);

    // Assert
    assert_eq!(
        error,
        ConfigError::Invalid {
            var: TRUSTED_PROXIES,
            reason: "ожидался список IP-адресов или CIDR через запятую"
        }
    );
}

#[test]
fn admin_port_needs_nothing_else_and_defaults() {
    // Arrange
    let empty = env(&[]);
    let custom = env(&[(ADMIN_PORT, "19464")]);
    let broken = env(&[(ADMIN_PORT, "x")]);

    // Act
    let results = [empty, custom, broken].map(|source| admin_port(&source));

    // Assert
    assert_eq!(
        results,
        [
            Ok(9464),
            Ok(19464),
            Err(ConfigError::Invalid {
                var: ADMIN_PORT,
                reason: "ожидалось целое число в допустимом диапазоне"
            })
        ]
    );
}
