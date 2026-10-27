use std::fmt;

const LEN: usize = 32;
const HEX_LEN: usize = LEN * 2;

/// Ошибка разбора hex-представления дайджеста.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DigestError {
    #[error("ожидалось 64 hex-символа, получено {0}")]
    InvalidLength(usize),
    #[error("недопустимый hex-символ {0:?}")]
    InvalidChar(char),
}

/// Дайджест SHA-256 (ключ содержимого в CAS).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Sha256([u8; LEN]);

impl Sha256 {
    /// Принимает hex любого регистра.
    pub fn from_hex(s: &str) -> Result<Self, DigestError> {
        let len = s.chars().count();
        if len != HEX_LEN {
            return Err(DigestError::InvalidLength(len));
        }
        let mut digits = [0u8; HEX_LEN];
        for (slot, c) in digits.iter_mut().zip(s.chars()) {
            *slot = c
                .to_digit(16)
                .and_then(|d| u8::try_from(d).ok())
                .ok_or(DigestError::InvalidChar(c))?;
        }
        let mut bytes = [0u8; LEN];
        let (pairs, _) = digits.as_chunks::<2>();
        for (byte, [hi, lo]) in bytes.iter_mut().zip(pairs) {
            *byte = (hi << 4) + lo;
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; LEN] {
        &self.0
    }
}

impl From<[u8; LEN]> for Sha256 {
    fn from(bytes: [u8; LEN]) -> Self {
        Self(bytes)
    }
}

impl fmt::Display for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

impl fmt::Debug for Sha256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sha256({self})")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

    #[test]
    fn known_vector_is_parsed_and_printed_back() {
        // Arrange
        let hex = EMPTY_SHA256;

        // Act
        let printed = Sha256::from_hex(hex).unwrap().to_string();

        // Assert
        assert_eq!(printed, hex);
    }

    #[test]
    fn uppercase_input_is_printed_in_lowercase() {
        // Arrange
        let upper = EMPTY_SHA256.to_ascii_uppercase();

        // Act
        let printed = Sha256::from_hex(&upper).unwrap().to_string();

        // Assert
        assert_eq!(printed, EMPTY_SHA256);
    }

    #[test]
    fn parsed_bytes_match_hex() {
        // Arrange
        let hex = EMPTY_SHA256;

        // Act
        let digest = Sha256::from_hex(hex).unwrap();

        // Assert
        assert_eq!(digest.as_bytes()[..4], [0xe3, 0xb0, 0xc4, 0x42]);
    }

    #[test]
    fn shorter_input_is_rejected_by_length() {
        // Arrange
        let hex = &EMPTY_SHA256[..63];

        // Act
        let result = Sha256::from_hex(hex);

        // Assert
        assert_eq!(result.unwrap_err(), DigestError::InvalidLength(63));
    }

    #[test]
    fn longer_input_is_rejected_by_length() {
        // Arrange
        let hex = format!("{EMPTY_SHA256}0");

        // Act
        let result = Sha256::from_hex(&hex);

        // Assert
        assert_eq!(result.unwrap_err(), DigestError::InvalidLength(65));
    }

    #[test]
    fn non_hex_character_is_rejected() {
        // Arrange
        let hex = format!("g{}", &EMPTY_SHA256[1..]);

        // Act
        let result = Sha256::from_hex(&hex);

        // Assert
        assert_eq!(result.unwrap_err(), DigestError::InvalidChar('g'));
    }

    #[test]
    fn bytes_below_0x10_are_zero_padded_and_round_trip() {
        // Arrange
        let digest = Sha256::from([0x0f; 32]);

        // Act
        let printed = digest.to_string();

        // Assert
        assert_eq!(
            (printed.len(), Sha256::from_hex(&printed)),
            (64, Ok(digest))
        );
    }

    #[test]
    fn debug_shows_full_lowercase_hex() {
        // Arrange
        let digest = Sha256::from([0xab; 32]);

        // Act
        let debug = format!("{digest:?}");

        // Assert
        assert_eq!(debug, format!("Sha256({})", "ab".repeat(32)));
    }
}
