//! SSRF guard: DNS-резолвер с проверкой адресов и решения по redirect-hop.

use std::error::Error;
use std::net::SocketAddr;

use crane_domain::proxy::{EgressPolicy, EgressRefusal};
use reqwest::Url;
use reqwest::dns::{Addrs, Name, Resolve, Resolving};

/// Сколько redirect допускается (как `Policy::limited` в reqwest). crates.io и зеркала редиректят максимум
/// на CDN (один-два hop); длиннее цепочка — это петля или атака, ответ 3xx станет `Unavailable`.
pub(crate) const MAX_REDIRECTS: usize = 5;

/// Решение по очередному redirect-hop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Hop {
    Follow,
    /// Цепочка слишком длинная: 3xx возвращается как есть.
    Stop,
    Refuse(EgressRefusal),
}

/// Каждый hop проходит те же проверки хоста и схемы, что и исходная база: иначе redirect уводит за allowlist.
pub(crate) fn hop(policy: &EgressPolicy, url: &Url, previous: usize) -> Hop {
    // `previous` включает исходный запрос, поэтому первый redirect приходит с `previous = 1`.
    if previous > MAX_REDIRECTS {
        return Hop::Stop;
    }
    let Some(host) = url.host_str() else {
        // Хоста нет; оператору хватает схемы (`data:`, `mailto:`). Остальное URL в журнал не кладём:
        // в query бывают подписи и токены.
        return Hop::Refuse(EgressRefusal::HostNotAllowed(format!("{}:", url.scheme())));
    };
    match policy.check_target(url.scheme(), host) {
        Ok(()) => Hop::Follow,
        Err(refusal) => Hop::Refuse(refusal),
    }
}

/// Ошибка резолва без адресов: отличается от отказа политики, это сбой, а не событие для `SecOps`.
#[derive(Debug, thiserror::Error)]
#[error("DNS не вернул адресов для «{0}»")]
struct NoAddresses(String);

/// Fail-closed: один непубличный адрес отклоняет весь резолв. Фильтрация молча скрыла бы отравленный
/// или split-horizon DNS, а это событие для `SecOps`. Возвращаемые адреса подключает сам reqwest,
/// поэтому проверяется именно то, к чему реально идёт connect.
fn vet(
    policy: &EgressPolicy,
    name: &str,
    addrs: Vec<SocketAddr>,
) -> Result<Vec<SocketAddr>, Box<dyn Error + Send + Sync>> {
    if addrs.is_empty() {
        return Err(Box::new(NoAddresses(name.to_owned())));
    }
    for addr in &addrs {
        policy.check_address(addr.ip())?;
    }
    Ok(addrs)
}

#[derive(Debug)]
pub(crate) struct GuardedResolver {
    pub(crate) policy: EgressPolicy,
}

impl Resolve for GuardedResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let policy = self.policy.clone();
        Box::pin(async move {
            let found = tokio::net::lookup_host((name.as_str(), 0)).await?;
            let addrs = vet(&policy, name.as_str(), found.collect())?;
            Ok(Box::new(addrs.into_iter()) as Addrs)
        })
    }
}

/// Отказ guard внутри цепочки причин: reqwest заворачивает ошибку резолвера и redirect-политики
/// в connect-/redirect-ошибку, поэтому идём по всей цепочке `source()`.
pub(crate) fn refusal(error: &(dyn Error + 'static)) -> Option<EgressRefusal> {
    let mut current = Some(error);
    while let Some(err) = current {
        if let Some(found) = err.downcast_ref::<EgressRefusal>() {
            return Some(found.clone());
        }
        current = err.source();
    }
    None
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use super::*;

    fn policy() -> EgressPolicy {
        EgressPolicy::new(["index.crates.io", "static.crates.io"], false)
    }

    fn url(raw: &str) -> Url {
        Url::parse(raw).expect("URL в тестовых данных валиден")
    }

    fn addr(ip: [u8; 4]) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::from(ip)), 0)
    }

    #[test]
    fn hop_follows_allowed_hosts_and_refuses_others() {
        // Arrange
        let cases = [
            ("https://static.crates.io/x", 1, Hop::Follow),
            (
                "https://evil.example/x",
                1,
                Hop::Refuse(EgressRefusal::HostNotAllowed("evil.example".to_owned())),
            ),
            (
                "http://static.crates.io/x",
                1,
                Hop::Refuse(EgressRefusal::InsecureScheme),
            ),
            (
                "ftp://static.crates.io/x",
                1,
                Hop::Refuse(EgressRefusal::UnsupportedScheme("ftp".to_owned())),
            ),
            ("https://static.crates.io/x", MAX_REDIRECTS, Hop::Follow),
            ("https://static.crates.io/x", MAX_REDIRECTS + 1, Hop::Stop),
        ];

        // Act
        let hops = cases
            .clone()
            .map(|(raw, previous, _)| hop(&policy(), &url(raw), previous));

        // Assert
        assert_eq!(hops, cases.map(|(_, _, expected)| expected));
    }

    #[test]
    fn hop_refuses_url_without_host() {
        // Arrange
        let target = url("data:text/plain,hello?token=secret");

        // Act
        let decision = hop(&policy(), &target, 1);

        // Assert
        assert_eq!(
            decision,
            Hop::Refuse(EgressRefusal::HostNotAllowed("data:".to_owned()))
        );
    }

    #[test]
    fn vet_rejects_whole_answer_when_any_address_is_not_public() {
        // Arrange
        let answer = vec![addr([93, 184, 216, 34]), addr([127, 0, 0, 1])];

        // Act
        let result = vet(&policy(), "index.crates.io", answer);

        // Assert
        assert_eq!(
            result.err().and_then(|error| refusal(error.as_ref())),
            Some(EgressRefusal::NonPublicAddress(IpAddr::V4(
                Ipv4Addr::LOCALHOST
            )))
        );
    }

    #[test]
    fn vet_passes_all_public_addresses_through() {
        // Arrange
        let answer = vec![addr([93, 184, 216, 34]), addr([8, 8, 8, 8])];

        // Act
        let result = vet(&policy(), "index.crates.io", answer.clone());

        // Assert
        assert_eq!(result.ok(), Some(answer));
    }

    #[test]
    fn vet_treats_empty_answer_as_failure_not_refusal() {
        // Arrange
        let answer = Vec::new();

        // Act
        let result = vet(&policy(), "index.crates.io", answer);

        // Assert
        assert_eq!(
            result
                .err()
                .map(|error| (error.to_string(), refusal(error.as_ref()))),
            Some((
                "DNS не вернул адресов для «index.crates.io»".to_owned(),
                None
            ))
        );
    }

    #[test]
    fn refusal_is_found_deep_in_the_source_chain() {
        // Arrange
        #[derive(Debug, thiserror::Error)]
        #[error("connect")]
        struct Outer(#[source] Box<dyn Error + Send + Sync>);
        let nested = Outer(Box::new(Outer(Box::new(EgressRefusal::InsecureScheme))));

        // Act
        let found = refusal(&nested);

        // Assert
        assert_eq!(found, Some(EgressRefusal::InsecureScheme));
    }
}
