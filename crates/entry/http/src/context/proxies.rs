//! Доверенные прокси и выбор адреса клиента.

use std::fmt;
use std::net::IpAddr;

/// Список доверенных прокси (`CRANE_TRUSTED_PROXIES`): IP или CIDR, IPv4 и IPv6.
/// По умолчанию пуст: тогда заголовкам `X-Forwarded-For` и `X-Request-Id` не верит никто.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TrustedProxies {
    nets: Vec<Net>,
}

/// Сеть хранится как «адрес + маска», уже наложенные друг на друга: `10.1.2.3/24` и `10.1.2.0/24` равны.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Net {
    V4 { network: u32, mask: u32 },
    V6 { network: u128, mask: u128 },
}

/// Некорректная запись списка доверенных прокси.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProxyParseError {
    entry: String,
}

impl fmt::Display for ProxyParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "`{}` не IP-адрес и не CIDR", self.entry)
    }
}

impl std::error::Error for ProxyParseError {}

impl Net {
    fn parse(entry: &str) -> Option<Self> {
        let (addr, prefix) = match entry.split_once('/') {
            Some((addr, prefix)) => (addr, Some(prefix.parse::<u8>().ok()?)),
            None => (entry, None),
        };
        // Адрес без префикса — одиночный хост; IPv4-mapped приводится к IPv4,
        // как и адрес пира при проверке.
        let addr: IpAddr = addr.parse().ok()?;
        let addr = if prefix.is_none() {
            addr.to_canonical()
        } else {
            addr
        };
        match addr {
            IpAddr::V4(v4) => {
                let prefix = prefix.unwrap_or(32);
                (prefix <= 32).then(|| {
                    let mask = u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0);
                    Self::V4 {
                        network: u32::from(v4) & mask,
                        mask,
                    }
                })
            }
            IpAddr::V6(v6) => {
                let prefix = prefix.unwrap_or(128);
                (prefix <= 128).then(|| {
                    let mask = u128::MAX.checked_shl(128 - u32::from(prefix)).unwrap_or(0);
                    Self::V6 {
                        network: u128::from(v6) & mask,
                        mask,
                    }
                })
            }
        }
    }

    fn contains(self, ip: IpAddr) -> bool {
        match (self, ip) {
            (Self::V4 { network, mask }, IpAddr::V4(v4)) => u32::from(v4) & mask == network,
            (Self::V6 { network, mask }, IpAddr::V6(v6)) => u128::from(v6) & mask == network,
            _ => false,
        }
    }
}

impl TrustedProxies {
    /// Список через запятую; пустая строка — пустой список.
    pub fn parse(raw: &str) -> Result<Self, ProxyParseError> {
        if raw.trim().is_empty() {
            return Ok(Self::default());
        }
        let nets = raw
            .split(',')
            .map(|item| {
                let item = item.trim();
                Net::parse(item).ok_or_else(|| ProxyParseError {
                    entry: item.to_owned(),
                })
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { nets })
    }

    #[must_use]
    pub fn contains(&self, ip: IpAddr) -> bool {
        // IPv4-mapped пир (::ffff:a.b.c.d) на dual-stack сокете — это обычный IPv4.
        let ip = ip.to_canonical();
        self.nets.iter().any(|net| net.contains(ip))
    }

    /// Адрес клиента. Пир вне списка — сам клиент, `X-Forwarded-For` ему не верим.
    /// Пир из списка: идём по заголовку справа налево мимо своих прокси; первый чужой
    /// адрес и есть клиент (левые записи подделывает кто угодно). Если все записи свои, берём
    /// самую правую разобранную: её дописал ближайший доверенный прокси, а левые вписал клиент
    /// (при широкой подсети клиент внутри неё подделал бы адрес). Если разобрать нечего — пир.
    #[must_use]
    pub(super) fn client_ip(&self, peer: IpAddr, forwarded_for: &str) -> IpAddr {
        let peer = peer.to_canonical();
        if !self.contains(peer) {
            return peer;
        }
        let hops: Vec<IpAddr> = forwarded_for
            .split(',')
            .filter_map(|entry| entry.trim().parse::<IpAddr>().ok())
            .map(|hop| hop.to_canonical())
            .collect();
        hops.iter()
            .rev()
            .find(|hop| !self.contains(**hop))
            .or_else(|| hops.last())
            .copied()
            .unwrap_or(peer)
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "в тестах падение expect — это падение теста"
)]
mod tests {
    use super::*;

    fn ip(raw: &str) -> IpAddr {
        raw.parse().expect("адрес в тесте")
    }

    fn proxies(raw: &str) -> TrustedProxies {
        TrustedProxies::parse(raw).expect("список в тесте")
    }

    #[test]
    fn cidr_membership_holds_at_range_edges() {
        // Arrange
        let cases = [
            ("10.0.0.0/24", "10.0.0.0", true),
            ("10.0.0.0/24", "10.0.0.255", true),
            ("10.0.0.0/24", "10.0.1.0", false),
            ("10.0.0.0/24", "9.255.255.255", false),
            ("10.0.0.7/24", "10.0.0.200", true),
            ("0.0.0.0/0", "203.0.113.9", true),
            ("192.0.2.1/32", "192.0.2.1", true),
            ("192.0.2.1/32", "192.0.2.2", false),
            ("192.0.2.1", "192.0.2.1", true),
            ("192.0.2.1", "192.0.2.2", false),
            ("2001:db8::/32", "2001:db8:ffff::1", true),
            ("2001:db8::/32", "2001:db9::1", false),
            ("::/0", "2001:db8::1", true),
            ("2001:db8::1/128", "2001:db8::1", true),
            ("2001:db8::1/128", "2001:db8::2", false),
            ("2001:db8::1", "2001:db8::1", true),
            ("10.0.0.0/8", "2001:db8::1", false),
            ("2001:db8::/32", "10.0.0.1", false),
        ];

        // Act
        let actual = cases.map(|(list, peer, _)| proxies(list).contains(ip(peer)));

        // Assert
        assert_eq!(actual, cases.map(|(_, _, expected)| expected));
    }

    #[test]
    fn ipv4_mapped_peer_matches_ipv4_entries() {
        // Arrange
        let trusted = proxies("10.0.0.0/8,192.0.2.1");
        let cases = ["::ffff:10.1.2.3", "::ffff:192.0.2.1", "::ffff:11.0.0.1"];

        // Act
        let actual = cases.map(|peer| trusted.contains(ip(peer)));

        // Assert
        assert_eq!(actual, [true, true, false]);
    }

    #[test]
    fn list_accepts_several_entries_and_blank_means_empty() {
        // Arrange
        let cases = [
            ("", "10.0.0.1"),
            ("   ", "10.0.0.1"),
            ("10.0.0.1, 192.0.2.0/24 ,2001:db8::/32", "192.0.2.77"),
            ("10.0.0.1, 192.0.2.0/24 ,2001:db8::/32", "2001:db8::5"),
        ];

        // Act
        let actual = cases.map(|(list, peer)| proxies(list).contains(ip(peer)));

        // Assert
        assert_eq!(actual, [false, false, true, true]);
    }

    #[test]
    fn rejection_names_the_offending_entry_for_the_operator() {
        // Arrange
        let raw = "10.0.0.1, 10.0.0.0/33";

        // Act
        let message = TrustedProxies::parse(raw).map_err(|error| error.to_string());

        // Assert
        assert_eq!(
            message,
            Err("`10.0.0.0/33` не IP-адрес и не CIDR".to_owned())
        );
    }

    #[test]
    fn invalid_entries_are_rejected() {
        // Arrange
        let cases = [
            "nonsense",
            "10.0.0.0/33",
            "2001:db8::/129",
            "10.0.0.0/",
            "10.0.0.0/-1",
            "10.0.0.0/x",
            "10.0.0.1,",
            ",10.0.0.1",
            "10.0.0.1,,10.0.0.2",
            "10.0.0.256",
        ];

        // Act
        let rejected = cases.map(|raw| TrustedProxies::parse(raw).is_err());

        // Assert
        assert_eq!(rejected, [true; 10]);
    }

    #[test]
    fn client_ip_is_chosen_by_peer_trust_and_forwarding_chain() {
        // Arrange
        let trusted = proxies("10.0.0.0/8");
        let cases = [
            // Недоверенный пир: заголовок игнорируется.
            ("203.0.113.5", "198.51.100.1", "203.0.113.5"),
            // Два доверенных прокси: первый чужой справа — клиент, подделка слева не важна.
            ("10.0.0.1", "6.6.6.6, 203.0.113.9, 10.0.0.2", "203.0.113.9"),
            // Мусор пропускается.
            ("10.0.0.1", "garbage, 203.0.113.9, nope", "203.0.113.9"),
            // Все записи свои: самая правая разобранная, левую мог вписать клиент.
            ("10.0.0.1", "10.0.0.5, 10.0.0.6", "10.0.0.6"),
            ("10.0.0.1", "10.9.9.9, 10.5.5.5", "10.5.5.5"),
            ("10.0.0.1", "10.9.9.9, garbage, 10.5.5.5, nope", "10.5.5.5"),
            // Ничего не разобралось или заголовка нет: пир.
            ("10.0.0.1", "garbage", "10.0.0.1"),
            ("10.0.0.1", "", "10.0.0.1"),
            // Адрес с портом не разбирается и пропускается.
            ("10.0.0.1", "203.0.113.9:4000", "10.0.0.1"),
            // Пир в mapped-форме остаётся доверенным.
            ("::ffff:10.0.0.1", "203.0.113.9", "203.0.113.9"),
        ];

        // Act
        let actual = cases.map(|(peer, xff, _)| trusted.client_ip(ip(peer), xff));

        // Assert
        assert_eq!(actual, cases.map(|(_, _, expected)| ip(expected)));
    }

    #[test]
    fn exact_proxy_addresses_stop_a_client_inside_the_network_from_forging_the_chain() {
        // Arrange
        let wide = proxies("10.0.0.0/8");
        let exact = proxies("10.0.0.1/32");
        let xff = "6.6.6.6, 10.5.5.5";

        // Act
        let chosen = [&wide, &exact].map(|trusted| trusted.client_ip(ip("10.0.0.1"), xff));

        // Assert
        // Широкая подсеть считает «своим» и реального клиента 10.5.5.5, и идёт дальше влево,
        // к подделке; точный адрес прокси останавливается на клиенте.
        assert_eq!(chosen, [ip("6.6.6.6"), ip("10.5.5.5")]);
    }
}
