use aws_lc_rs::aead::{Aad, NONCE_LEN, Nonce};
use crane_domain::settings::{Secret, Version};

use super::key::SettingsKey;

/// Длина тега аутентификации AES-GCM.
const TAG_LEN: usize = 16;

/// Почему секрет не запечатан или не открыт. Только имя настройки и отпечатки ключей:
/// ни байтов, ни значений в сообщениях нет.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CryptoError {
    #[error(
        "секрет настройки {setting} запечатан ключом {stored_key_id}, а задан ключ {current_key_id}: \
         верните прежний CRANE_SETTINGS_KEY_FILE или перезапишите секрет"
    )]
    WrongKey {
        setting: String,
        stored_key_id: String,
        current_key_id: String,
    },
    #[error("секрет настройки {setting} повреждён или принадлежит другой строке или версии")]
    Corrupted { setting: String },
    #[error(
        "секрет настройки {setting} лежит в БД, а ключ шифрования не задан (CRANE_SETTINGS_KEY_FILE)"
    )]
    MissingKey { setting: String },
    #[error("не удалось зашифровать секрет настройки {setting}")]
    SealFailed { setting: String },
}

/// Связывает шифртекст со строкой: имя настройки, версия и отпечаток ключа. Перенесённый в
/// другую строку или оставшийся от старой версии шифртекст не открывается.
fn associated_data(setting: &str, version: Version, key_id: &str) -> Aad<Vec<u8>> {
    let mut data = Vec::with_capacity(setting.len() + 1 + 8 + key_id.len());
    data.extend_from_slice(setting.as_bytes());
    data.push(0);
    data.extend_from_slice(&version.get().to_be_bytes());
    data.extend_from_slice(key_id.as_bytes());
    Aad::from(data)
}

/// Запечатывает секрет: `nonce (12) || шифртекст || тег (16)`.
pub(in crate::settings) fn seal(
    key: &SettingsKey,
    setting: &str,
    version: Version,
    plaintext: &Secret,
) -> Result<Vec<u8>, CryptoError> {
    let mut body = plaintext.expose().as_bytes().to_vec();
    let nonce = key
        .cipher()
        .seal_in_place_append_tag(associated_data(setting, version, key.id()), &mut body)
        .map_err(|_| CryptoError::SealFailed {
            setting: setting.to_owned(),
        })?;
    let mut sealed = Vec::with_capacity(NONCE_LEN + body.len());
    sealed.extend_from_slice(nonce.as_ref());
    sealed.extend_from_slice(&body);
    Ok(sealed)
}

/// Открывает конверт из строки `setting` версии `version`, запечатанный ключом `stored_key_id`.
pub(in crate::settings) fn open(
    key: &SettingsKey,
    setting: &str,
    version: Version,
    stored_key_id: &str,
    sealed: &[u8],
) -> Result<Secret, CryptoError> {
    let corrupted = || CryptoError::Corrupted {
        setting: setting.to_owned(),
    };
    // Чужой ключ распознаётся по отпечатку без попытки расшифровки: так оператор видит причину.
    if stored_key_id != key.id() {
        return Err(CryptoError::WrongKey {
            setting: setting.to_owned(),
            stored_key_id: stored_key_id.to_owned(),
            current_key_id: key.id().to_owned(),
        });
    }
    if sealed.len() < NONCE_LEN + TAG_LEN {
        return Err(corrupted());
    }
    let (nonce, body) = sealed.split_at(NONCE_LEN);
    let nonce = Nonce::try_assume_unique_for_key(nonce).map_err(|_| corrupted())?;
    let mut buffer = body.to_vec();
    let plaintext = key
        .cipher()
        .open_in_place(
            nonce,
            associated_data(setting, version, key.id()),
            &mut buffer,
        )
        .map_err(|_| corrupted())?;
    String::from_utf8(plaintext.to_vec())
        .map(Secret::new)
        .map_err(|_| corrupted())
}

#[cfg(test)]
mod tests {
    use super::super::key::tests::{encoded, key};
    use super::*;

    const SETTING: &str = "token";
    const PLAIN: &str = "crs_plain-secret-value";

    fn version(n: u64) -> Version {
        Version::new(n).expect("версия не ноль")
    }

    fn sealed(fill: u8) -> Vec<u8> {
        seal(
            &key(fill),
            SETTING,
            version(1),
            &Secret::new(PLAIN.to_owned()),
        )
        .expect("seal")
    }

    fn opened(result: Result<Secret, CryptoError>) -> Result<String, CryptoError> {
        result.map(|secret| secret.expose().to_owned())
    }

    #[test]
    fn open_returns_sealed_value_with_same_setting_version_and_key() {
        // Arrange
        let envelope = sealed(1);

        // Act
        let result = open(&key(1), SETTING, version(1), key(1).id(), &envelope);

        // Assert
        assert_eq!(opened(result), Ok(PLAIN.to_owned()));
    }

    #[test]
    fn open_returns_empty_secret_sealed_without_plaintext() {
        // Arrange
        let envelope = seal(&key(1), SETTING, version(1), &Secret::new(String::new()));

        // Act
        let result = open(
            &key(1),
            SETTING,
            version(1),
            key(1).id(),
            &envelope.expect("seal"),
        );

        // Assert
        assert_eq!(opened(result), Ok(String::new()));
    }

    #[test]
    fn open_rejects_envelope_moved_to_other_setting() {
        // Arrange
        let envelope = sealed(1);

        // Act
        let result = open(&key(1), "other", version(1), key(1).id(), &envelope);

        // Assert
        assert_eq!(
            opened(result),
            Err(CryptoError::Corrupted {
                setting: "other".to_owned()
            })
        );
    }

    #[test]
    fn open_rejects_envelope_of_other_version() {
        // Arrange
        let envelope = sealed(1);

        // Act
        let result = open(&key(1), SETTING, version(2), key(1).id(), &envelope);

        // Assert
        assert_eq!(
            opened(result),
            Err(CryptoError::Corrupted {
                setting: SETTING.to_owned()
            })
        );
    }

    #[test]
    fn open_reports_both_fingerprints_for_other_key() {
        // Arrange
        let envelope = sealed(1);

        // Act
        let result = open(&key(2), SETTING, version(1), key(1).id(), &envelope);

        // Assert
        assert_eq!(
            opened(result),
            Err(CryptoError::WrongKey {
                setting: SETTING.to_owned(),
                stored_key_id: key(1).id().to_owned(),
                current_key_id: key(2).id().to_owned(),
            })
        );
    }

    #[test]
    fn open_rejects_tampered_ciphertext_byte() {
        // Arrange
        let mut envelope = sealed(1);
        envelope[NONCE_LEN] ^= 1;

        // Act
        let result = open(&key(1), SETTING, version(1), key(1).id(), &envelope);

        // Assert
        assert_eq!(
            opened(result),
            Err(CryptoError::Corrupted {
                setting: SETTING.to_owned()
            })
        );
    }

    #[test]
    fn open_rejects_envelope_shorter_than_nonce_and_tag() {
        // Arrange
        let short = vec![0_u8; NONCE_LEN + TAG_LEN - 1];

        // Act
        let result = open(&key(1), SETTING, version(1), key(1).id(), &short);

        // Assert
        assert_eq!(
            opened(result),
            Err(CryptoError::Corrupted {
                setting: SETTING.to_owned()
            })
        );
    }

    #[test]
    fn seal_of_same_value_twice_gives_different_envelopes() {
        // Arrange
        let (key, secret) = (key(1), Secret::new(PLAIN.to_owned()));

        // Act
        let envelopes = [
            seal(&key, SETTING, version(1), &secret),
            seal(&key, SETTING, version(1), &secret),
        ];

        // Assert
        assert_ne!(envelopes[0], envelopes[1]);
    }

    #[test]
    fn seal_output_contains_no_plaintext() {
        // Arrange
        let envelope = sealed(1);

        // Act
        let leaked = envelope
            .windows(PLAIN.len())
            .any(|window| window == PLAIN.as_bytes());

        // Assert
        assert!(!leaked);
    }

    #[test]
    fn error_messages_contain_neither_plaintext_nor_key_material() {
        // Arrange
        let errors = [
            CryptoError::WrongKey {
                setting: SETTING.to_owned(),
                stored_key_id: key(1).id().to_owned(),
                current_key_id: key(2).id().to_owned(),
            },
            CryptoError::Corrupted {
                setting: SETTING.to_owned(),
            },
            CryptoError::MissingKey {
                setting: SETTING.to_owned(),
            },
            CryptoError::SealFailed {
                setting: SETTING.to_owned(),
            },
        ];
        let key_text = encoded(1).expose().to_owned();

        // Act
        let leaks = errors.map(|error| {
            let message = error.to_string();
            message.contains(PLAIN) || message.contains(&key_text)
        });

        // Assert
        assert_eq!(leaks, [false; 4]);
    }
}
