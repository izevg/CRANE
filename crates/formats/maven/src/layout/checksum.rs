use crane_domain::registry::{DigestError, Digests};
use md5::Md5;
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha512};

/// Алгоритм контрольной суммы: ось представления, ортогональная ресурсу. Один и тот же
/// файл или документ отдаётся как есть и с суффиксом `.md5`/`.sha1`/`.sha256`/`.sha512`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Algorithm {
    Md5,
    Sha1,
    Sha256,
    Sha512,
}

impl Algorithm {
    pub const ALL: [Self; 4] = [Self::Md5, Self::Sha1, Self::Sha256, Self::Sha512];

    /// Суффикс имени файла без точки.
    #[must_use]
    pub const fn suffix(self) -> &'static str {
        match self {
            Self::Md5 => "md5",
            Self::Sha1 => "sha1",
            Self::Sha256 => "sha256",
            Self::Sha512 => "sha512",
        }
    }

    /// Длина суммы в hex-символах.
    #[must_use]
    pub const fn hex_len(self) -> usize {
        match self {
            Self::Md5 => 32,
            Self::Sha1 => 40,
            Self::Sha256 => 64,
            Self::Sha512 => 128,
        }
    }

    /// Алгоритм по суффиксу имени файла (`sha1` -> `Sha1`).
    pub(crate) fn from_suffix(suffix: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|alg| alg.suffix() == suffix)
    }

    /// Сумма байтов документа (`maven-metadata.xml`) в hex нижнего регистра. Файлы версий
    /// берут сумму из дайджестов каталога ([`Algorithm::of`]), а не пересчитывают.
    #[must_use]
    pub fn hash(self, bytes: &[u8]) -> String {
        match self {
            Self::Md5 => format!("{:x}", Md5::digest(bytes)),
            Self::Sha1 => format!("{:x}", Sha1::digest(bytes)),
            Self::Sha256 => format!("{:x}", Sha256::digest(bytes)),
            Self::Sha512 => format!("{:x}", Sha512::digest(bytes)),
        }
    }

    /// Сумма файла версии из дайджестов каталога, посчитанных при записи, в hex нижнего регистра.
    #[must_use]
    pub fn of(self, digests: &Digests) -> String {
        match self {
            Self::Md5 => digests.md5.to_string(),
            Self::Sha1 => digests.sha1.to_string(),
            Self::Sha256 => digests.sha256.to_string(),
            Self::Sha512 => digests.sha512.to_string(),
        }
    }

    /// Присланная клиентом сумма: первый токен тела (дальше resolver ждёт имя файла или перевод
    /// строки), hex любого регистра, нужной длины. Возвращает hex нижнего регистра для сверки
    /// с вычисленной суммой.
    pub fn parse_declared(self, body: &str) -> Result<String, DigestError> {
        let token = body.split_whitespace().next().unwrap_or_default();
        let actual = token.chars().count();
        if actual != self.hex_len() {
            return Err(DigestError::InvalidLength {
                expected: self.hex_len(),
                actual,
            });
        }
        match token.chars().find(|c| !c.is_ascii_hexdigit()) {
            Some(bad) => Err(DigestError::InvalidChar(bad)),
            None => Ok(token.to_ascii_lowercase()),
        }
    }
}
