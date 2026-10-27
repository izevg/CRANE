use std::fmt;

use super::{Sha1, Sha256, Sha512};

/// Контрольная сумма с явным алгоритмом: то, что заявил клиент или upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Checksum {
    Sha256(Sha256),
    Sha512(Sha512),
    Sha1(Sha1),
}

impl fmt::Display for Checksum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sha256(d) => write!(f, "sha256:{d}"),
            Self::Sha512(d) => write!(f, "sha512:{d}"),
            Self::Sha1(d) => write!(f, "sha1:{d}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_prefixes_hex_with_algorithm() {
        // Arrange
        let cases = [
            Checksum::Sha256(Sha256::from([0xab; 32])),
            Checksum::Sha512(Sha512::from([0x01; 64])),
            Checksum::Sha1(Sha1::from([0xff; 20])),
        ];

        // Act
        let actual = cases.map(|c| c.to_string());

        // Assert
        assert_eq!(
            actual,
            [
                format!("sha256:{}", "ab".repeat(32)),
                format!("sha512:{}", "01".repeat(64)),
                format!("sha1:{}", "ff".repeat(20)),
            ]
        );
    }

    #[test]
    fn same_bytes_under_different_algorithms_are_not_equal() {
        // Arrange
        let sha1 = Checksum::Sha1(Sha1::from([7; 20]));
        let sha256 = Checksum::Sha256(Sha256::from([7; 32]));

        // Act
        let equal = sha1 == sha256;

        // Assert
        assert!(!equal);
    }
}
