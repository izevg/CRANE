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
