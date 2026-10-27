use std::fmt;

/// Ошибка разбора hex-представления дайджеста.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DigestError {
    #[error("ожидалось {expected} hex-символов, получено {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("недопустимый hex-символ {0:?}")]
    InvalidChar(char),
}

const RADIX: u32 = 16;
const NIBBLE_BITS: u32 = 4;

/// Разбирает hex любого регистра ровно в `N` байт.
pub(super) fn parse<const N: usize>(s: &str) -> Result<[u8; N], DigestError> {
    let actual = s.chars().count();
    if actual != N * 2 {
        return Err(DigestError::InvalidLength {
            expected: N * 2,
            actual,
        });
    }
    let mut bytes = [0u8; N];
    let mut chars = s.chars();
    for byte in &mut bytes {
        let (Some(hi), Some(lo)) = (chars.next(), chars.next()) else {
            break;
        };
        *byte = (nibble(hi)? << NIBBLE_BITS) | nibble(lo)?;
    }
    Ok(bytes)
}

fn nibble(c: char) -> Result<u8, DigestError> {
    c.to_digit(RADIX)
        .and_then(|d| u8::try_from(d).ok())
        .ok_or(DigestError::InvalidChar(c))
}

/// Печатает байты строчным hex с ведущими нулями.
pub(super) fn write(bytes: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    bytes.iter().try_for_each(|b| write!(f, "{b:02x}"))
}
