//! Разбор ответа upstream: статус → вариант порта, `Cache-Control` и `Age` → подсказка свежести.

use std::time::Duration;

use crane_ports::proxy::ETag;
use reqwest::StatusCode;
use reqwest::header::{AGE, CACHE_CONTROL, ETAG, HeaderMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Classified {
    Found {
        etag: Option<ETag>,
        max_age: Option<Duration>,
    },
    NotModified {
        max_age: Option<Duration>,
    },
    NotFound,
    /// Upstream отказывается обслуживать нас (429, 403).
    Throttled,
    /// Любой другой статус: порт трактует его как недоступность upstream.
    Other(StatusCode),
}

pub(crate) fn classify(status: StatusCode, headers: &HeaderMap) -> Classified {
    match status {
        StatusCode::OK => Classified::Found {
            // ETag отдаём как есть (включая `W/` и кавычки): upstream сам сравнивает его побайтово.
            etag: headers
                .get(ETAG)
                .and_then(|value| value.to_str().ok())
                .map(|raw| ETag::new(raw.to_owned())),
            max_age: freshness_hint(headers),
        },
        StatusCode::NOT_MODIFIED => Classified::NotModified {
            max_age: freshness_hint(headers),
        },
        // 410 и 451 — снятие пакета (в том числе по закону): как 404, иначе stale-if-error
        // заморозил бы удалённый документ на неделю. Исходный статус остаётся в журнале egress.
        StatusCode::NOT_FOUND | StatusCode::GONE | StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS => {
            Classified::NotFound
        }
        StatusCode::TOO_MANY_REQUESTS | StatusCode::FORBIDDEN => Classified::Throttled,
        other => Classified::Other(other),
    }
}

/// Срок свежести из `Cache-Control` за вычетом `Age`.
///
/// `s-maxage` приоритетнее `max-age`: proxy — общий кэш. `no-cache`, `no-store` и `private`
/// отменяют подсказку целиком (квалифицированные формы тоже: проще и безопаснее). Невалидное число
/// игнорирует свою директиву.
pub(crate) fn freshness_hint(headers: &HeaderMap) -> Option<Duration> {
    let mut s_maxage = None;
    let mut max_age = None;
    for value in headers.get_all(CACHE_CONTROL) {
        let Ok(line) = value.to_str() else {
            continue;
        };
        for directive in line.split(',') {
            let (name, argument) = directive
                .split_once('=')
                .map_or((directive, None), |(name, argument)| (name, Some(argument)));
            match name.trim().to_ascii_lowercase().as_str() {
                "no-cache" | "no-store" | "private" => return None,
                "s-maxage" => s_maxage = s_maxage.or_else(|| seconds(argument)),
                "max-age" => max_age = max_age.or_else(|| seconds(argument)),
                _ => {}
            }
        }
    }
    let term = s_maxage.or(max_age)?;
    Some(Duration::from_secs(term.saturating_sub(age(headers))))
}

/// delta-seconds, значение возможно в кавычках.
fn seconds(argument: Option<&str>) -> Option<u64> {
    argument?.trim().trim_matches('"').parse().ok()
}

/// Сколько ответ уже пролежал в кэшах выше по цепочке; невалидный `Age` считается нулём.
fn age(headers: &HeaderMap) -> u64 {
    headers
        .get(AGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|raw| raw.trim().parse().ok())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderName, HeaderValue};

    use super::*;

    type Pairs = &'static [(&'static str, &'static str)];

    fn headers(pairs: Pairs) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.append(
                HeaderName::from_static(name),
                HeaderValue::from_static(value),
            );
        }
        map
    }

    #[test]
    fn freshness_hint_reads_cache_control_and_subtracts_age() {
        // Arrange
        let cases: [(Pairs, Option<u64>); 15] = [
            (&[("cache-control", "max-age=600")], Some(600)),
            (&[("cache-control", "max-age=600, s-maxage=300")], Some(300)),
            (&[("cache-control", "s-maxage=300, max-age=600")], Some(300)),
            (&[("cache-control", "max-age=\"600\"")], Some(600)),
            (&[("cache-control", "Max-Age=600")], Some(600)),
            (&[("cache-control", "public , max-age = 600")], Some(600)),
            (&[("cache-control", "max-age=600, no-cache")], None),
            (&[("cache-control", "no-store, max-age=600")], None),
            (&[("cache-control", "private, max-age=600")], None),
            (&[("cache-control", "max-age=abc")], None),
            (&[("cache-control", "max-age=abc, s-maxage=100")], Some(100)),
            (
                &[("cache-control", "max-age=600"), ("age", "100")],
                Some(500),
            ),
            (&[("cache-control", "max-age=60"), ("age", "100")], Some(0)),
            (
                &[("cache-control", "max-age=600"), ("age", "soon")],
                Some(600),
            ),
            (
                &[
                    ("cache-control", "public"),
                    ("cache-control", "max-age=600"),
                ],
                Some(600),
            ),
        ];

        // Act
        let hints = cases.map(|(pairs, _)| freshness_hint(&headers(pairs)));

        // Assert
        assert_eq!(hints, cases.map(|(_, secs)| secs.map(Duration::from_secs)));
    }

    #[test]
    fn freshness_hint_is_none_without_cache_control() {
        // Arrange
        let map = headers(&[("age", "10")]);

        // Act
        let hint = freshness_hint(&map);

        // Assert
        assert_eq!(hint, None);
    }

    #[test]
    fn classify_maps_status_to_port_variant() {
        // Arrange
        let map = headers(&[("etag", "W/\"v1\""), ("cache-control", "max-age=60")]);
        let hint = Some(Duration::from_secs(60));
        let cases = [
            (
                StatusCode::OK,
                Classified::Found {
                    etag: Some(ETag::new("W/\"v1\"".to_owned())),
                    max_age: hint,
                },
            ),
            (
                StatusCode::NOT_MODIFIED,
                Classified::NotModified { max_age: hint },
            ),
            (StatusCode::NOT_FOUND, Classified::NotFound),
            (StatusCode::GONE, Classified::NotFound),
            (
                StatusCode::UNAVAILABLE_FOR_LEGAL_REASONS,
                Classified::NotFound,
            ),
            (StatusCode::TOO_MANY_REQUESTS, Classified::Throttled),
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Classified::Other(StatusCode::INTERNAL_SERVER_ERROR),
            ),
            (StatusCode::FORBIDDEN, Classified::Throttled),
        ];

        // Act
        let classified = cases.clone().map(|(status, _)| classify(status, &map));

        // Assert
        assert_eq!(classified, cases.map(|(_, expected)| expected));
    }
}
