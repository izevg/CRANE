use aws_lc_rs::aead::{AES_256_GCM, RandomizedNonceKey};
use aws_lc_rs::digest::{SHA256, digest};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use crane_domain::settings::Secret;

/// Длина ключа AES-256 в байтах.
const KEY_LEN: usize = 32;
/// Сколько байт SHA-256 берётся в отпечаток: 8 байт дают 16 hex-символов.
const FINGERPRINT_BYTES: usize = 8;

/// Почему ключ не принят. В сообщениях только длина: содержимое файла ключа не раскрывается.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum KeyError {
    #[error("ключ шифрования не является base64 (standard, с выравниванием, без пробелов)")]
    NotBase64,
    #[error("ключ шифрования после base64 занимает {bytes} байт, нужно ровно 32")]
    WrongLength { bytes: usize },
}

/// Ключ шифрования секретов в БД (AES-256-GCM). Без `Debug`, `Display` и `Clone`.
#[allow(
    missing_debug_implementations,
    reason = "Debug ключа запрещён: значение не должно попасть в журнал"
)]
pub struct SettingsKey {
    key: RandomizedNonceKey,
    id: String,
}

impl SettingsKey {
    /// Разбирает ключ из base64 (`openssl rand -base64 32`). Перевод строки по краям файла
    /// обрезает вызывающий: лишние пробелы здесь - ошибка.
    ///
    /// # Errors
    /// [`KeyError::NotBase64`] или [`KeyError::WrongLength`], если байт не 32.
    pub fn from_base64(encoded: &Secret) -> Result<Self, KeyError> {
        let bytes = STANDARD
            .decode(encoded.expose())
            .map_err(|_| KeyError::NotBase64)?;
        let wrong_length = KeyError::WrongLength { bytes: bytes.len() };
        if bytes.len() != KEY_LEN {
            return Err(wrong_length);
        }
        let key = RandomizedNonceKey::new(&AES_256_GCM, &bytes).map_err(|_| wrong_length)?;
        Ok(Self {
            key,
            id: fingerprint(&bytes),
        })
    }

    /// Отпечаток ключа (16 hex-символов SHA-256): по нему видно, каким ключом запечатана строка.
    /// Не секрет, по нему ключ не восстановить.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    pub(super) const fn cipher(&self) -> &RandomizedNonceKey {
        &self.key
    }
}

fn fingerprint(key: &[u8]) -> String {
    digest(&SHA256, key).as_ref()[..FINGERPRINT_BYTES]
        .iter()
        .flat_map(|byte| [byte >> 4, byte & 0x0f])
        .filter_map(|nibble| char::from_digit(u32::from(nibble), 16))
        .collect()
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    /// Ключ из `fill`, повторённого 32 раза, в base64.
    pub(in crate::settings) fn encoded(fill: u8) -> Secret {
        Secret::new(STANDARD.encode([fill; KEY_LEN]))
    }

    pub(in crate::settings) fn key(fill: u8) -> SettingsKey {
        SettingsKey::from_base64(&encoded(fill)).expect("ключ валиден")
    }

    #[test]
    fn from_base64_requires_valid_base64_of_exactly_32_bytes() {
        // Arrange
        let cases = [
            STANDARD.encode([1_u8; 32]),
            STANDARD.encode([1_u8; 31]),
            STANDARD.encode([1_u8; 33]),
            "не base64!".to_owned(),
            format!(" {}", STANDARD.encode([1_u8; 32])),
        ];

        // Act
        let actual = cases.map(|text| SettingsKey::from_base64(&Secret::new(text)).err());

        // Assert
        assert_eq!(
            actual,
            [
                None,
                Some(KeyError::WrongLength { bytes: 31 }),
                Some(KeyError::WrongLength { bytes: 33 }),
                Some(KeyError::NotBase64),
                Some(KeyError::NotBase64),
            ]
        );
    }

    #[test]
    fn key_id_is_sha256_prefix_of_key_bytes() {
        // Arrange
        let zeros = key(0);

        // Act
        let id = zeros.id();

        // Assert
        // SHA-256 от 32 нулевых байт: 66687aadf862bd77...
        assert_eq!(id, "66687aadf862bd77");
    }

    #[test]
    fn key_id_differs_between_keys() {
        // Arrange
        let (first, second) = (key(1), key(2));

        // Act
        let same = first.id() == second.id();

        // Assert
        assert!(!same);
    }

    #[test]
    fn key_error_messages_do_not_contain_key_text() {
        // Arrange
        let text = STANDARD.encode([1_u8; 31]);
        let errors = [
            SettingsKey::from_base64(&Secret::new(text.clone())).err(),
            SettingsKey::from_base64(&Secret::new(format!("{text}!"))).err(),
        ];

        // Act
        let leaks = errors.map(|error| error.is_some_and(|e| e.to_string().contains(&text)));

        // Assert
        assert_eq!(leaks, [false, false]);
    }
}
