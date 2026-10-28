use zeroize::Zeroizing;

/// Секрет (токен, URL базы, ключ доступа). Без `Debug`, `Display`, `Clone` и `PartialEq`:
/// случайно попасть в журнал, быть размноженным или сравненным не в постоянное время он не может.
/// Память обнуляется при `Drop`: значение не переживает секрет в освобождённой куче.
#[allow(
    missing_debug_implementations,
    reason = "Debug секрета запрещён: значение не должно попасть в журнал"
)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    /// Единственный способ прочитать значение: вызов виден в ревью и при поиске по коду.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}

/// Куда указывает секрет-URL, без учётных данных: `scheme://host[:port]`.
///
/// Нужен там, где значение секрета показывать нельзя, а адрес — можно и нужно: журнал аудита и
/// консоль должны отвечать, куда смотрит кэш или хранилище (смена адреса — способ подмены данных).
/// Хост берётся как записан: без DNS и без приведения к IP; IPv6 остаётся в квадратных скобках;
/// порт есть, только если он задан в значении (умолчание схемы не подставляется). Userinfo, путь,
/// запрос и фрагмент не возвращаются никогда. Не URL, пустой хост и нецифровой порт дают `None`
/// (в том числе список хостов через запятую): лучше не показать адрес, чем показать часть.
#[must_use]
pub fn locator(value: &str) -> Option<String> {
    let (scheme, rest) = value.split_once("://")?;
    if !is_scheme(scheme) {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    // Пароль может содержать `@`: граница userinfo — последний.
    let hostport = authority
        .rsplit_once('@')
        .map_or(authority, |(_, tail)| tail);
    let (host, port) = split_host_port(hostport)?;
    if host.is_empty() || host == "[]" || host.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return None;
    }
    Some(match port {
        Some(port) => format!("{scheme}://{host}:{port}"),
        None => format!("{scheme}://{host}"),
    })
}

fn is_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
}

/// Хост (для IPv6 вместе со скобками) и порт, если он задан. `None` — порт не цифровой.
fn split_host_port(hostport: &str) -> Option<(&str, Option<&str>)> {
    let (host, tail) = if hostport.starts_with('[') {
        let end = hostport.find(']')? + 1;
        hostport.split_at(end)
    } else {
        hostport.split_at(hostport.find(':').unwrap_or(hostport.len()))
    };
    if tail.is_empty() {
        return Some((host, None));
    }
    let port = tail.strip_prefix(':')?;
    (!port.is_empty() && port.chars().all(|c| c.is_ascii_digit())).then_some((host, Some(port)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locator_keeps_scheme_and_host_and_drops_everything_else() {
        // Arrange
        let cases = [
            ("valkey://u:p@h:6379", Some("valkey://h:6379")),
            ("valkey://:pw@h", Some("valkey://h")),
            ("redis://h", Some("redis://h")),
            ("s3://bucket/path?x", Some("s3://bucket")),
            ("https://h/a#frag", Some("https://h")),
            (
                "postgres://u:p@w@h:5432/db?sslmode=x",
                Some("postgres://h:5432"),
            ),
        ];

        // Act
        let actual = cases.map(|(value, _)| locator(value));

        // Assert
        assert_eq!(
            actual,
            cases.map(|(_, expected)| expected.map(str::to_owned))
        );
    }

    #[test]
    fn locator_keeps_the_host_as_written() {
        // Arrange
        let cases = [
            "valkey://Cache.Internal",
            "valkey://127.0.0.1:6379",
            "valkey://localhost",
        ];

        // Act
        let actual = cases.map(locator);

        // Assert
        assert_eq!(
            actual,
            [
                Some("valkey://Cache.Internal".to_owned()),
                Some("valkey://127.0.0.1:6379".to_owned()),
                Some("valkey://localhost".to_owned())
            ]
        );
    }

    #[test]
    fn locator_keeps_ipv6_in_brackets_and_the_port_only_when_written() {
        // Arrange
        let cases = [
            "redis://[::1]:6379",
            "redis://u:p@[::1]",
            "redis://[2001:db8::1]/0",
        ];

        // Act
        let actual = cases.map(locator);

        // Assert
        assert_eq!(
            actual,
            [
                Some("redis://[::1]:6379".to_owned()),
                Some("redis://[::1]".to_owned()),
                Some("redis://[2001:db8::1]".to_owned())
            ]
        );
    }

    #[test]
    fn locator_does_not_add_the_default_port_of_the_scheme() {
        // Arrange
        let cases = ["https://h", "valkey://h", "postgres://u:p@h/db"];

        // Act
        let actual = cases.map(locator);

        // Assert
        assert_eq!(
            actual,
            [
                Some("https://h".to_owned()),
                Some("valkey://h".to_owned()),
                Some("postgres://h".to_owned())
            ]
        );
    }

    #[test]
    fn locator_refuses_what_is_not_a_url_with_a_host() {
        // Arrange
        let cases = [
            "",
            "hunter2",
            "[::1]:6379",
            "h:6379",
            "valkey://",
            "valkey://u:p@",
            "valkey://:6379",
            "valkey://[]:1",
            "valkey://h:",
            "valkey://h:port",
            "valkey://h1:1,h2:2",
            "valkey://[::1",
            "valkey://[::1]x",
            "1x://h",
            "://h",
            "val key://h",
            "valkey://h h",
        ];

        // Act
        let actual = cases.map(locator);

        // Assert
        assert_eq!(actual.to_vec(), vec![None; cases.len()]);
    }

    #[test]
    fn locator_never_contains_credentials() {
        // Arrange
        let value = "valkey://admin:s3cr3t-pw@cache.internal:6379/0?password=also-secret";

        // Act
        let shown = locator(value).unwrap_or_default();

        // Assert
        assert_eq!(
            (
                shown.contains("admin"),
                shown.contains("s3cr3t"),
                shown.contains("also-secret")
            ),
            (false, false, false)
        );
    }
}
