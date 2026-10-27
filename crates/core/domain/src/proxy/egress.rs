use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use thiserror::Error;

use super::path::{PathError, UpstreamPath};

/// Схема исходящего запроса. `Http` допускается только явным разрешением в политике.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheme {
    Https,
    Http,
}

impl Scheme {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Https => "https",
            Self::Http => "http",
        }
    }
}

/// Защита от SSRF для исходящих запросов proxy: точный allowlist хостов, схема и класс адреса.
/// Wildcard-хостов нет сознательно: каждый upstream перечисляется явно.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EgressPolicy {
    hosts: Vec<String>,
    allow_http: bool,
}

/// База upstream, прошедшая allowlist. Конструктор приватный: получить можно только через [`EgressPolicy::admit`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamBase {
    scheme: Scheme,
    host: String,
    port: Option<u16>,
    prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum EgressRefusal {
    #[error("хост «{0}» не входит в allowlist")]
    HostNotAllowed(String),
    #[error("схема http запрещена политикой")]
    InsecureScheme,
    #[error("схема «{0}» не поддерживается")]
    UnsupportedScheme(String),
    #[error("адрес {0} не публичный")]
    NonPublicAddress(IpAddr),
    #[error("недопустимый префикс пути upstream: {0}")]
    InvalidPrefix(#[from] PathError),
}

impl EgressPolicy {
    /// Хосты нормализуются так же, как проверяемые: ASCII lowercase, без хвостовой точки.
    pub fn new(hosts: impl IntoIterator<Item = impl AsRef<str>>, allow_http: bool) -> Self {
        Self {
            hosts: hosts
                .into_iter()
                .map(|host| normalize_host(host.as_ref()))
                .collect(),
            allow_http,
        }
    }

    /// Конфиг proxy-репозитория → проверенная база. Вызывает entry при сборке.
    pub fn admit(
        &self,
        scheme: &str,
        host: &str,
        port: Option<u16>,
        prefix: &str,
    ) -> Result<UpstreamBase, EgressRefusal> {
        let scheme = self.check_scheme(scheme)?;
        self.check_host(host)?;
        Ok(UpstreamBase {
            scheme,
            host: normalize_host(host),
            port,
            prefix: normalize_prefix(prefix)?,
        })
    }

    /// Каждый redirect-hop: redirect не должен вести на хост вне allowlist или на небезопасную схему.
    /// Порт не участвует в allowlist: порты upstream бывают нестандартные, хост и адрес проверяются отдельно.
    pub fn check_target(&self, scheme: &str, host: &str) -> Result<(), EgressRefusal> {
        self.check_scheme(scheme)?;
        self.check_host(host)
    }

    /// Каждый адрес после DNS-резолва, прямо перед connect: allowlist по имени не защищает от DNS на внутренний адрес.
    pub fn check_address(&self, ip: IpAddr) -> Result<(), EgressRefusal> {
        if is_public(ip) {
            Ok(())
        } else {
            Err(EgressRefusal::NonPublicAddress(ip))
        }
    }

    fn check_scheme(&self, scheme: &str) -> Result<Scheme, EgressRefusal> {
        if scheme.eq_ignore_ascii_case("https") {
            Ok(Scheme::Https)
        } else if scheme.eq_ignore_ascii_case("http") {
            if self.allow_http {
                Ok(Scheme::Http)
            } else {
                Err(EgressRefusal::InsecureScheme)
            }
        } else {
            Err(EgressRefusal::UnsupportedScheme(scheme.to_owned()))
        }
    }

    fn check_host(&self, host: &str) -> Result<(), EgressRefusal> {
        let normalized = normalize_host(host);
        if !self.hosts.contains(&normalized) {
            return Err(EgressRefusal::HostNotAllowed(normalized));
        }
        // IP-литерал в allowlist не отменяет проверку класса адреса: резолва нет, значит и проверки позже не будет.
        match normalized.parse::<IpAddr>() {
            Ok(ip) => self.check_address(ip),
            Err(_) => Ok(()),
        }
    }
}

impl UpstreamBase {
    #[must_use]
    pub const fn scheme(&self) -> Scheme {
        self.scheme
    }

    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    #[must_use]
    pub const fn port(&self) -> Option<u16> {
        self.port
    }

    /// `{scheme}://{host}[:port]/{prefix}{path}`. Префикс пуст или оканчивается `/` и не начинается с `/`,
    /// поэтому путь всегда остаётся после authority и не может сменить хост.
    #[must_use]
    pub fn url(&self, path: &UpstreamPath) -> String {
        let host = &self.host;
        // IPv6-литерал в URL обязан быть в квадратных скобках.
        let host = if host.contains(':') {
            format!("[{host}]")
        } else {
            host.clone()
        };
        let port = self
            .port
            .map_or_else(String::new, |port| format!(":{port}"));
        format!(
            "{}://{host}{port}/{}{}",
            self.scheme.as_str(),
            self.prefix,
            path.as_str()
        )
    }
}

/// ASCII lowercase, без хвостовой точки (FQDN) и без скобок IPv6-литерала.
fn normalize_host(raw: &str) -> String {
    let unbracketed = raw
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(raw);
    unbracketed.trim_end_matches('.').to_ascii_lowercase()
}

/// Префикс → пустая строка или `a/b/`. Правила сегментов те же, что у пути: `..`, `?`, `#`, `\`, `:` отвергаются.
fn normalize_prefix(raw: &str) -> Result<String, PathError> {
    let trimmed = raw.trim_matches('/');
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    UpstreamPath::parse(trimmed).map(|path| format!("{}/", path.as_str()))
}

/// Диапазоны IPv4 вне `std`-предикатов. Источник: IANA IPv4 Special-Purpose Address Registry.
const V4_NON_PUBLIC: [(Ipv4Addr, u8); 5] = [
    (Ipv4Addr::UNSPECIFIED, 8),         // «this network»
    (Ipv4Addr::new(100, 64, 0, 0), 10), // CGNAT, в том числе 100.100.100.200 (Alibaba metadata)
    (Ipv4Addr::new(192, 0, 0, 0), 24),  // IETF protocol assignments
    (Ipv4Addr::new(198, 18, 0, 0), 15), // benchmarking
    (Ipv4Addr::new(240, 0, 0, 0), 4),   // reserved (и 255.255.255.255)
];

/// Диапазоны IPv6 вне `std`-предикатов. Источник: IANA IPv6 Special-Purpose Address Registry.
const V6_NON_PUBLIC: [(Ipv6Addr, u8); 6] = [
    // IPv4-compatible (устарел, RFC 4291 §2.5.5.1), в том числе ::/128 и ::1/128; ::ffff:0:0/96 разворачивает `to_canonical`
    (Ipv6Addr::UNSPECIFIED, 96),
    (Ipv6Addr::new(0xfec0, 0, 0, 0, 0, 0, 0, 0), 10), // site-local (устарел)
    (Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0), 32), // documentation
    (Ipv6Addr::new(0x100, 0, 0, 0, 0, 0, 0, 0), 64),  // discard-only
    (Ipv6Addr::new(0x2001, 0, 0, 0, 0, 0, 0, 0), 32), // Teredo
    (Ipv6Addr::new(0x64, 0xff9b, 1, 0, 0, 0, 0, 0), 48), // local-use NAT64
];

/// Публичный (глобально маршрутизируемый) ли адрес. Нестабильные `is_global`/`is_shared` не используются.
fn is_public(ip: IpAddr) -> bool {
    // IPv4-mapped (::ffff:a.b.c.d) иначе обошёл бы все IPv4-правила.
    match ip.to_canonical() {
        IpAddr::V4(v4) => is_public_v4(v4),
        IpAddr::V6(v6) => is_public_v6(v6),
    }
}

fn is_public_v4(ip: Ipv4Addr) -> bool {
    let std_non_public = ip.is_loopback()
        || ip.is_private()
        || ip.is_link_local()
        || ip.is_multicast()
        || ip.is_broadcast()
        || ip.is_documentation()
        || ip.is_unspecified();
    !std_non_public
        && !V4_NON_PUBLIC
            .iter()
            .any(|&(net, len)| in_v4_prefix(ip, net, len))
}

fn is_public_v6(ip: Ipv6Addr) -> bool {
    let std_non_public = ip.is_unspecified()
        || ip.is_loopback()
        || ip.is_multicast()
        || ip.is_unique_local()
        || ip.is_unicast_link_local();
    if std_non_public
        || V6_NON_PUBLIC
            .iter()
            .any(|&(net, len)| in_v6_prefix(ip, net, len))
    {
        return false;
    }
    // NAT64 и 6to4 несут IPv4 внутри: публичность определяет вложенный адрес.
    embedded_v4(ip).is_none_or(is_public_v4)
}

/// Вложенный IPv4 для `64:ff9b::/96` (RFC 6052, последние 32 бита) и `2002::/16` (6to4, биты 16..48).
fn embedded_v4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let bits = u128::from(ip);
    if in_v6_prefix(ip, Ipv6Addr::new(0x64, 0xff9b, 0, 0, 0, 0, 0, 0), 96) {
        // Младшие 32 бита: усечение намеренное.
        #[allow(
            clippy::cast_possible_truncation,
            reason = "берём ровно младшие 32 бита"
        )]
        Some(Ipv4Addr::from(bits as u32))
    } else if in_v6_prefix(ip, Ipv6Addr::new(0x2002, 0, 0, 0, 0, 0, 0, 0), 16) {
        #[allow(
            clippy::cast_possible_truncation,
            reason = "берём ровно 32 бита после префикса 2002"
        )]
        Some(Ipv4Addr::from((bits >> 80) as u32))
    } else {
        None
    }
}

fn in_v4_prefix(ip: Ipv4Addr, net: Ipv4Addr, len: u8) -> bool {
    let shift = 32 - u32::from(len);
    u32::from(ip) >> shift == u32::from(net) >> shift
}

fn in_v6_prefix(ip: Ipv6Addr, net: Ipv6Addr, len: u8) -> bool {
    let shift = 128 - u32::from(len);
    u128::from(ip) >> shift == u128::from(net) >> shift
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]
mod tests {
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
    fn admit_exposes_normalized_parts() {
        // Arrange
        let policy = policy();

        // Act
        let base = policy
            .admit("HTTPS", "Index.Crates.IO.", Some(443), "")
            .unwrap();

        // Assert
        assert_eq!(
            (base.scheme(), base.host(), base.port()),
            (Scheme::Https, "index.crates.io", Some(443))
        );
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
}
