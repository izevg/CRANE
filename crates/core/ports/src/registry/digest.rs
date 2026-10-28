use crane_domain::registry::{Digests, Md5, Sha1, Sha256, Sha512};
use sha1::Digest;

/// Потоковый подсчёт всех дайджестов за один проход по телу.
#[derive(Debug, Default)]
pub struct Digester {
    sha256: sha2::Sha256,
    sha512: sha2::Sha512,
    sha1: sha1::Sha1,
    md5: md5::Md5,
    size: u64,
}

impl Digester {
    pub fn update(&mut self, chunk: &[u8]) {
        self.sha256.update(chunk);
        self.sha512.update(chunk);
        self.sha1.update(chunk);
        self.md5.update(chunk);
        self.size += chunk.len() as u64;
    }

    #[must_use]
    pub fn finish(self) -> Digests {
        Digests {
            sha256: Sha256::from(<[u8; 32]>::from(self.sha256.finalize())),
            sha512: Sha512::from(<[u8; 64]>::from(self.sha512.finalize())),
            sha1: Sha1::from(<[u8; 20]>::from(self.sha1.finalize())),
            md5: Md5::from(<[u8; 16]>::from(self.md5.finalize())),
            size: self.size,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HELLO_SHA1: &str = "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d";
    const HELLO_MD5: &str = "5d41402abc4b2a76b9719d911017c592";

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
    fn digester_computes_known_sha1_md5_and_size() {
        // Arrange
        let expected = (
            Sha1::from_hex(HELLO_SHA1).unwrap(),
            Md5::from_hex(HELLO_MD5).unwrap(),
            5,
        );

        // Act
        let digests = hello();

        // Assert
        assert_eq!((digests.sha1, digests.md5, digests.size), expected);
    }
}
