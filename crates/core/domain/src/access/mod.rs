//! Кто и в каком контексте обращается к ядру.

use std::net::IpAddr;

use uuid::Uuid;

/// Максимальная длина идентификатора запроса.
const REQUEST_ID_MAX_LEN: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub id: String,
    pub kind: PrincipalKind,
}

/// Фоновые действия самого CRANE: первый старт, перевыпуск токена.
pub const SYSTEM_PRINCIPAL_ID: &str = "crane:system";
/// Записи мастера первого запуска под bootstrap-токеном.
pub const BOOTSTRAP_PRINCIPAL_ID: &str = "crane:bootstrap";

impl Principal {
    #[must_use]
    pub fn system() -> Self {
        Self::robot(SYSTEM_PRINCIPAL_ID)
    }

    #[must_use]
    pub fn bootstrap() -> Self {
        Self::robot(BOOTSTRAP_PRINCIPAL_ID)
    }

    fn robot(id: &str) -> Self {
        Self {
            id: id.to_owned(),
            kind: PrincipalKind::Robot,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrincipalKind {
    Human,
    Robot,
    Workload,
    Agent,
}

/// Идентификатор запроса: свой `UUIDv7` или принятый от доверенного прокси.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(String);

impl RequestId {
    /// 1..=128 символов, только видимый ASCII 0x21..=0x7E; иначе `None` (заголовок мог прийти от клиента).
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        // Строгий набор символов не даёт внедрить в журнал пробелы, переводы строк и управляющие байты.
        let valid = (1..=REQUEST_ID_MAX_LEN).contains(&raw.len())
            && raw.bytes().all(|byte| (0x21..=0x7E).contains(&byte));
        valid.then(|| Self(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<Uuid> for RequestId {
    fn from(id: Uuid) -> Self {
        Self(id.hyphenated().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestContext {
    pub request_id: RequestId,
    pub trace_id: Option<String>,
    pub client_ip: Option<IpAddr>,
    pub user_agent: Option<String>,
}

/// Аутентифицированный участник с контекстом запроса. Без `Option`: use case нельзя вызвать анонимно.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caller {
    pub principal: Principal,
    pub request: RequestContext,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_only_printable_ascii_within_length_limit() {
        // Arrange
        let long_ok = "a".repeat(128);
        let too_long = "a".repeat(129);
        let cases = [
            ("a", true),
            (long_ok.as_str(), true),
            ("abc-123_X.~", true),
            ("", false),
            (too_long.as_str(), false),
            ("a b", false),
            ("é", false),
            ("\u{7f}", false),
            ("a\tb", false),
        ];

        // Act
        let accepted = cases.map(|(raw, _)| RequestId::parse(raw).is_some());

        // Assert
        assert_eq!(accepted, cases.map(|(_, expected)| expected));
    }

    #[test]
    fn request_id_from_uuid_is_hyphenated_lower_case() {
        // Arrange
        let uuid = Uuid::from_u128(0xABCD_EF01_2345_6789_ABCD_EF01_2345_6789);

        // Act
        let id = RequestId::from(uuid);

        // Assert
        assert_eq!(id.as_str(), "abcdef01-2345-6789-abcd-ef0123456789");
    }
}
