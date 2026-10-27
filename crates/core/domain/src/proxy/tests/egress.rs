#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use super::*;

fn ip(raw: &str) -> IpAddr {
    raw.parse().unwrap()
}

fn policy() -> EgressPolicy {
    EgressPolicy::new(["index.crates.io", "registry.npmjs.org"], false)
}

#[test]
fn check_address_refuses_every_non_public_range() {
    // Arrange
    let non_public = [
        "0.0.0.0",
        "0.255.255.255",
        "10.0.0.1",
        "100.64.0.1",
        "100.100.100.200",
        "100.127.255.255",
        "127.0.0.1",
        "169.254.169.254",
        "172.16.0.1",
        "172.31.255.255",
        "192.0.0.1",
        "192.0.2.1",
        "192.168.1.1",
        "198.18.0.1",
        "198.19.255.255",
        "198.51.100.1",
        "203.0.113.1",
        "224.0.0.1",
        "239.255.255.255",
        "240.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "fc00::1",
        "fd00:ec2::254",
        "fe80::1",
        "fec0::1",
        "ff02::1",
        "2001:db8::1",
        "100::1",
        "2001::1",
        "64:ff9b:1::1",
        "::7f00:1",
        "::a00:1",
        "::ffff:127.0.0.1",
        "::ffff:10.0.0.1",
        "::ffff:169.254.169.254",
        "64:ff9b::a00:1",
        "64:ff9b::7f00:1",
        "2002:7f00:1::",
        "2002:a9fe:a9fe::",
        "2002:a00:808:808::",
    ];

    // Act
    let results = non_public.map(|raw| policy().check_address(ip(raw)));

    // Assert
    assert_eq!(
        results,
        non_public.map(|raw| Err(EgressRefusal::NonPublicAddress(ip(raw))))
    );
}

#[test]
fn check_address_allows_public_addresses_and_range_neighbours() {
    // Arrange
    let public = [
        "1.1.1.1",
        "151.101.0.1",
        "8.8.8.8",
        "100.63.255.255",
        "100.128.0.1",
        "172.15.255.255",
        "172.32.0.1",
        "198.17.255.255",
        "198.20.0.1",
        "192.0.1.1",
        "223.255.255.255",
        "2606:4700::1111",
        "2001:4860:4860::8888",
        "2001:1::1",
        "64:ff9b::101:101",
        "::ffff:8.8.8.8",
        "2002:808:808::",
    ];

    // Act
    let results = public.map(|raw| policy().check_address(ip(raw)));

    // Assert
    assert_eq!(results, public.map(|_| Ok(())));
}

#[test]
fn check_target_accepts_https_for_allowed_host() {
    // Arrange
    let policy = policy();

    // Act
    let result = policy.check_target("https", "index.crates.io");

    // Assert
    assert_eq!(result, Ok(()));
}

#[test]
fn check_target_normalizes_scheme_case_and_host_form() {
    // Arrange
    let policy = policy();
    let cases = [
        ("HTTPS", "index.crates.io"),
        ("https", "INDEX.Crates.IO"),
        ("https", "index.crates.io."),
    ];

    // Act
    let results = cases.map(|(scheme, host)| policy.check_target(scheme, host));

    // Assert
    assert_eq!(results, [Ok(()), Ok(()), Ok(())]);
}

#[test]
fn check_target_normalizes_hosts_given_to_the_policy() {
    // Arrange
    let policy = EgressPolicy::new(["Index.Crates.IO."], false);

    // Act
    let result = policy.check_target("https", "index.crates.io");

    // Assert
    assert_eq!(result, Ok(()));
}

#[test]
fn check_target_refuses_http_unless_explicitly_allowed() {
    // Arrange
    let strict = EgressPolicy::new(["mirror.local.example"], false);
    let lax = EgressPolicy::new(["mirror.local.example"], true);

    // Act
    let (refused, refused_upper, allowed) = (
        strict.check_target("http", "mirror.local.example"),
        strict.check_target("HTTP", "mirror.local.example"),
        lax.check_target("HTTP", "mirror.local.example"),
    );

    // Assert
    assert_eq!(
        (refused, refused_upper, allowed),
        (
            Err(EgressRefusal::InsecureScheme),
            Err(EgressRefusal::InsecureScheme),
            Ok(())
        )
    );
}

#[test]
fn check_target_refuses_unsupported_scheme() {
    // Arrange
    let policy = policy();

    // Act
    let result = policy.check_target("ftp", "index.crates.io");

    // Assert
    assert_eq!(
        result,
        Err(EgressRefusal::UnsupportedScheme("ftp".to_owned()))
    );
}

#[test]
fn check_target_refuses_hosts_outside_exact_allowlist() {
    // Arrange
    let policy = policy();
    let cases = [
        "evil.index.crates.io",
        "crates.io",
        "index.crates.io.evil.com",
        "index.crates.io@evil.com",
        "",
    ];

    // Act
    let results = cases.map(|host| policy.check_target("https", host));

    // Assert
    assert_eq!(
        results,
        cases.map(|host| Err(EgressRefusal::HostNotAllowed(host.to_owned())))
    );
}

#[test]
fn check_target_refuses_allowlisted_ip_literal_in_private_range() {
    // Arrange
    let policy = EgressPolicy::new(["10.0.0.1", "::1", "8.8.8.8"], false);

    // Act
    let (private_v4, loopback_v6, public_v4) = (
        policy.check_target("https", "10.0.0.1"),
        policy.check_target("https", "[::1]"),
        policy.check_target("https", "8.8.8.8"),
    );

    // Assert
    assert_eq!(
        (private_v4, loopback_v6, public_v4),
        (
            Err(EgressRefusal::NonPublicAddress(ip("10.0.0.1"))),
            Err(EgressRefusal::NonPublicAddress(ip("::1"))),
            Ok(())
        )
    );
}

#[test]
fn check_target_refuses_ip_literal_missing_from_allowlist() {
    // Arrange
    let policy = policy();

    // Act
    let result = policy.check_target("https", "[::1]");

    // Assert
    assert_eq!(result, Err(EgressRefusal::HostNotAllowed("::1".to_owned())));
}

#[test]
fn url_joins_base_and_path_under_host() {
    // Arrange
    let base = policy()
        .admit("https", "index.crates.io", None, "")
        .unwrap();
    let path = UpstreamPath::parse("se/rd/serde").unwrap();

    // Act
    let url = base.url(&path);

    // Assert
    assert_eq!(url, "https://index.crates.io/se/rd/serde");
}

#[test]
fn url_is_same_for_equivalent_prefix_spellings() {
    // Arrange
    let policy = policy();
    let path = UpstreamPath::parse("@babel/core").unwrap();
    let spellings = ["npm", "/npm/", "npm/", "/npm"];

    // Act
    let urls = spellings.map(|prefix| {
        policy
            .admit("https", "registry.npmjs.org", None, prefix)
            .unwrap()
            .url(&path)
    });

    // Assert
    assert_eq!(
        urls,
        spellings.map(|_| "https://registry.npmjs.org/npm/@babel/core".to_owned())
    );
}

#[test]
fn url_includes_port() {
    // Arrange
    let policy = EgressPolicy::new(["mirror.example"], true);
    let base = policy
        .admit("http", "Mirror.Example.", Some(8080), "")
        .unwrap();
    let path = UpstreamPath::parse("a").unwrap();

    // Act
    let url = base.url(&path);

    // Assert
    assert_eq!(url, "http://mirror.example:8080/a");
}

#[test]
fn url_brackets_ipv6_literal_host() {
    // Arrange
    let policy = EgressPolicy::new(["2606:4700::1111"], false);
    let base = policy
        .admit("https", "[2606:4700::1111]", Some(8443), "p")
        .unwrap();
    let path = UpstreamPath::parse("a").unwrap();

    // Act
    let url = base.url(&path);

    // Assert
    assert_eq!(url, "https://[2606:4700::1111]:8443/p/a");
}

#[test]
fn admit_refuses_host_outside_allowlist() {
    // Arrange
    let policy = policy();

    // Act
    let result = policy.admit("https", "evil.example", None, "");

    // Assert
    assert_eq!(
        result,
        Err(EgressRefusal::HostNotAllowed("evil.example".to_owned()))
    );
}

#[test]
fn admit_refuses_prefix_that_could_escape_the_base() {
    // Arrange
    let policy = policy();
    let bad_prefixes = ["a/../b", "a?x=1", "a#f", "a\\b", "a:b", "a//b", "%2e%2e"];

    // Act
    let refused = bad_prefixes.map(|prefix| {
        matches!(
            policy.admit("https", "index.crates.io", None, prefix),
            Err(EgressRefusal::InvalidPrefix(_))
        )
    });

    // Assert
    assert_eq!(refused, bad_prefixes.map(|_| true));
}
