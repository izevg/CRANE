use crane_domain::registry::{Checksum, Sha1, Sha256, Sha512};
use sha1::Digest;

/// Все дайджесты содержимого и его размер. SHA-256 — ключ в CAS, остальные нужны для сверки
/// с тем, что заявил клиент или upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Digests {
    pub sha256: Sha256,
    pub sha512: Sha512,
    pub sha1: Sha1,
    pub size: u64,
}

impl Digests {
    /// Совпадает ли ожидаемая сумма с посчитанной тем же алгоритмом.
    #[must_use]
    pub fn satisfies(&self, expected: &Checksum) -> bool {
        self.of_kind(expected) == *expected
    }

    /// Посчитанная сумма того же алгоритма, что и `like`.
    #[must_use]
    pub const fn of_kind(&self, like: &Checksum) -> Checksum {
        match like {
            Checksum::Sha256(_) => Checksum::Sha256(self.sha256),
            Checksum::Sha512(_) => Checksum::Sha512(self.sha512),
            Checksum::Sha1(_) => Checksum::Sha1(self.sha1),
        }
    }
}

/// Потоковый подсчёт всех дайджестов за один проход по телу.
#[derive(Debug, Default)]
pub struct Digester {
    sha256: sha2::Sha256,
    sha512: sha2::Sha512,
    sha1: sha1::Sha1,
    size: u64,
}

impl Digester {
    pub fn update(&mut self, chunk: &[u8]) {
        self.sha256.update(chunk);
        self.sha512.update(chunk);
        self.sha1.update(chunk);
        self.size += chunk.len() as u64;
    }

    #[must_use]
    pub fn finish(self) -> Digests {
        Digests {
            sha256: Sha256::from(<[u8; 32]>::from(self.sha256.finalize())),
            sha512: Sha512::from(<[u8; 64]>::from(self.sha512.finalize())),
            sha1: Sha1::from(<[u8; 20]>::from(self.sha1.finalize())),
            size: self.size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_SHA1: &str = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";

    fn digest_of(chunks: &[&[u8]]) -> Digests {
        let mut digester = Digester::default();
        for chunk in chunks {
            digester.update(chunk);
        }
        digester.finish()
    }

    fn hello() -> Digests {
        digest_of(&[b"hello"])
    }

    #[test]
    fn digester_hashes_chunks_as_one_stream() {
        // Arrange
        let chunks: [&[u8]; 2] = [b"hel", b"lo"];

        // Act
        let digests = digest_of(&chunks);

        // Assert
        assert_eq!(digests, hello());
    }

    #[test]
    fn digester_computes_known_sha1_and_size() {
        // Arrange
        let expected = (Sha1::from_hex(HELLO_SHA1).unwrap(), 5);

        // Act
        let digests = hello();

        // Assert
        assert_eq!((digests.sha1, digests.size), expected);
    }

    #[test]
    fn satisfies_compares_only_the_same_algorithm() {
        // Arrange
        let digests = hello();
        let cases = [
            Checksum::Sha256(digests.sha256),
            Checksum::Sha512(digests.sha512),
            Checksum::Sha1(digests.sha1),
            Checksum::Sha256(Sha256::from([0; 32])),
            Checksum::Sha512(Sha512::from([0; 64])),
            Checksum::Sha1(Sha1::from([0; 20])),
        ];

        // Act
        let actual = cases.map(|c| digests.satisfies(&c));

        // Assert
        assert_eq!(actual, [true, true, true, false, false, false]);
    }

    #[test]
    fn of_kind_returns_own_digest_of_requested_algorithm() {
        // Arrange
        let digests = hello();
        let cases = [
            Checksum::Sha256(Sha256::from([0; 32])),
            Checksum::Sha512(Sha512::from([0; 64])),
            Checksum::Sha1(Sha1::from([0; 20])),
        ];

        // Act
        let actual = cases.map(|c| digests.of_kind(&c));

        // Assert
        assert_eq!(
            actual,
            [
                Checksum::Sha256(digests.sha256),
                Checksum::Sha512(digests.sha512),
                Checksum::Sha1(digests.sha1),
            ]
        );
    }
}
