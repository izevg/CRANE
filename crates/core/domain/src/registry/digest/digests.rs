use super::{Checksum, Md5, Sha1, Sha256, Sha512};

/// Все дайджесты содержимого и его размер. SHA-256 — ключ в CAS, остальные нужны для сверки
/// с тем, что заявил клиент, и для отдачи контрольных сумм протоколами клиентов.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Digests {
    pub sha256: Sha256,
    pub sha512: Sha512,
    pub sha1: Sha1,
    pub md5: Md5,
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
            Checksum::Md5(_) => Checksum::Md5(self.md5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Digests {
        Digests {
            sha256: Sha256::from([1; 32]),
            sha512: Sha512::from([2; 64]),
            sha1: Sha1::from([3; 20]),
            md5: Md5::from([4; 16]),
            size: 5,
        }
    }

    #[test]
    fn satisfies_compares_only_the_same_algorithm() {
        // Arrange
        let digests = sample();
        let cases = [
            Checksum::Sha256(digests.sha256),
            Checksum::Sha512(digests.sha512),
            Checksum::Sha1(digests.sha1),
            Checksum::Md5(digests.md5),
            Checksum::Sha256(Sha256::from([0; 32])),
            Checksum::Sha512(Sha512::from([0; 64])),
            Checksum::Sha1(Sha1::from([0; 20])),
            Checksum::Md5(Md5::from([0; 16])),
        ];

        // Act
        let actual = cases.map(|c| digests.satisfies(&c));

        // Assert
        assert_eq!(actual, [true, true, true, true, false, false, false, false]);
    }

    #[test]
    fn of_kind_returns_own_digest_of_requested_algorithm() {
        // Arrange
        let digests = sample();
        let cases = [
            Checksum::Sha256(Sha256::from([0; 32])),
            Checksum::Sha512(Sha512::from([0; 64])),
            Checksum::Sha1(Sha1::from([0; 20])),
            Checksum::Md5(Md5::from([0; 16])),
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
                Checksum::Md5(digests.md5),
            ]
        );
    }
}
