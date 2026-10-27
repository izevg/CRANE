//! Дайджесты содержимого: SHA-256 (ключ CAS), SHA-512 и SHA-1 (контрольные суммы форматов).

mod checksum;
mod hex;

use std::fmt;

pub use checksum::Checksum;
pub use hex::DigestError;

/// Объявляет тип дайджеста фиксированной длины с общим разбором и выводом hex.
macro_rules! digest {
    ($(#[$meta:meta])* $name:ident, $len:expr) => {
        $(#[$meta])*
        #[derive(Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name([u8; $len]);

        impl $name {
            /// Принимает hex любого регистра.
            pub fn from_hex(s: &str) -> Result<Self, DigestError> {
                hex::parse(s).map(Self)
            }

            #[must_use]
            pub const fn as_bytes(&self) -> &[u8; $len] {
                &self.0
            }
        }

        impl From<[u8; $len]> for $name {
            fn from(bytes: [u8; $len]) -> Self {
                Self(bytes)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                hex::write(&self.0, f)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self)
            }
        }
    };
}

digest!(
    /// Дайджест SHA-256 (ключ содержимого в CAS).
    Sha256,
    32
);
digest!(
    /// Дайджест SHA-512 (`dist.integrity` npm).
    Sha512,
    64
);
digest!(
    /// Дайджест SHA-1 (`dist.shasum` npm). Не криптостойкий: только сверка с заявленным клиентом.
    Sha1,
    20
);

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
        assert_eq!(
            result.unwrap_err(),
            DigestError::InvalidLength {
                expected: 64,
                actual: 63
            }
        );
    }

    #[test]
    fn longer_input_is_rejected_by_length() {
        // Arrange
        let hex = format!("{EMPTY_SHA256}0");

        // Act
        let result = Sha256::from_hex(&hex);

        // Assert
        assert_eq!(
            result.unwrap_err(),
            DigestError::InvalidLength {
                expected: 64,
                actual: 65
            }
        );
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
    fn sha512_and_sha1_round_trip_through_hex() {
        // Arrange
        let sha512 = "ab".repeat(64);
        let sha1 = "0f".repeat(20);

        // Act
        let printed = (
            Sha512::from_hex(&sha512).map(|d| d.to_string()),
            Sha1::from_hex(&sha1).map(|d| d.to_string()),
        );

        // Assert
        assert_eq!(printed, (Ok(sha512), Ok(sha1)));
    }

    #[test]
    fn sha1_rejects_sha256_length() {
        // Arrange
        let hex = EMPTY_SHA256;

        // Act
        let result = Sha1::from_hex(hex);

        // Assert
        assert_eq!(
            result.unwrap_err(),
            DigestError::InvalidLength {
                expected: 40,
                actual: 64
            }
        );
    }
}
