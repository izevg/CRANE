use fred::types::config::Config;

/// Допустимые схемы адреса. `redis*` намеренно не принимаются: адаптер говорит с Valkey.
const SCHEMES: [&str; 6] = [
    "valkey",
    "valkeys",
    "valkey-cluster",
    "valkeys-cluster",
    "valkey-sentinel",
    "valkeys-sentinel",
];

/// Разобранный адрес Valkey. Без `Debug`/`Display`: в URL бывает пароль.
#[allow(
    missing_debug_implementations,
    reason = "Debug конфигурации fred печатает пароль"
)]
pub struct Endpoint(pub(super) Config);

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum EndpointError {
    /// Только схема, без остального URL.
    #[error("схема `{scheme}` не поддерживается: ожидалось valkey(s)[-cluster|-sentinel]://")]
    Scheme { scheme: String },
    /// Текст ошибки fred не выводится: он может цитировать URL.
    #[error("адрес Valkey не разобран")]
    Invalid,
}

impl Endpoint {
    /// Разбирает `valkey[s][-cluster|-sentinel]://[user:password@]host[:port][/db]`.
    pub fn parse(url: &str) -> Result<Self, EndpointError> {
        let (scheme, _) = url.split_once("://").ok_or(EndpointError::Invalid)?;
        if !SCHEMES.contains(&scheme) {
            return Err(EndpointError::Scheme {
                scheme: scheme.to_owned(),
            });
        }
        Config::from_url(url)
            .map(Self)
            .map_err(|_| EndpointError::Invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_only_valkey_schemes() {
        // Arrange
        let cases = [
            "redis://h:6379",
            "rediss://h:6379",
            "http://h",
            "valkey://h:6379",
            "valkeys://h:6379",
            "valkey://h",
            "valkey://user:secret@h:6379/3",
            "valkey-cluster://h:6379",
            "valkey-sentinel://h:26379?sentinelServiceName=main",
        ];

        // Act
        let actual = cases.map(|url| Endpoint::parse(url).err());

        // Assert
        let rejected = |scheme: &str| {
            Some(EndpointError::Scheme {
                scheme: scheme.to_owned(),
            })
        };
        assert_eq!(
            actual,
            [
                rejected("redis"),
                rejected("rediss"),
                rejected("http"),
                None,
                None,
                None,
                None,
                None,
                None
            ]
        );
    }

    #[test]
    fn parse_rejects_text_without_scheme_separator() {
        // Arrange
        let cases = ["", "valkey", "valkey:/h", "h:6379"];

        // Act
        let actual = cases.map(|url| Endpoint::parse(url).err());

        // Assert
        assert_eq!(actual, [const { Some(EndpointError::Invalid) }; 4]);
    }

    #[test]
    fn parse_rejects_malformed_url_with_known_scheme_without_quoting_it() {
        // Arrange
        let url = "valkey://user:secret@[::1";

        // Act
        let error = Endpoint::parse(url).err();

        // Assert
        assert_eq!(error, Some(EndpointError::Invalid));
    }

    #[test]
    fn scheme_error_does_not_quote_url_rest() {
        // Arrange
        let url = "redis://user:secret@h:6379";

        // Act
        let message = Endpoint::parse(url).err().map(|error| error.to_string());

        // Assert
        assert_eq!(
            message.as_deref().map(|text| text.contains("secret")),
            Some(false)
        );
    }
}
