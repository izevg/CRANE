//! Bootstrap-токен: `crb_` и 43 символа base64url от 32 случайных байт (256 бит, раздел 20.5).

use aws_lc_rs::rand;
use crane_domain::onboarding::TokenHash;
use crane_domain::settings::Secret;

use crate::config::BOOTSTRAP_PREFIX;

const ENTROPY_BYTES: usize = 32;
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const BITS_PER_SYMBOL: u32 = 6;
const BITS_PER_BYTE: u32 = 8;
const SYMBOL_MASK: u32 = 0b11_1111;

#[derive(Debug, thiserror::Error)]
#[error("источник случайных чисел недоступен")]
pub(crate) struct RandomError;

/// Токен входа в мастер. Без `Debug`, `Display` и `Clone` (внутри `Secret`): единственное место,
/// где значение выходит наружу, - строка `crane::bootstrap` (`announce`).
pub(crate) struct BootstrapToken(Secret);

impl BootstrapToken {
    pub(crate) fn generate() -> Result<Self, RandomError> {
        let mut bytes = [0_u8; ENTROPY_BYTES];
        rand::fill(&mut bytes).map_err(|_| RandomError)?;
        Ok(Self(Secret::new(format!(
            "{BOOTSTRAP_PREFIX}{}",
            base64url(&bytes)
        ))))
    }

    /// Токен, заданный оператором; формат уже проверил разбор конфигурации.
    pub(crate) const fn provided(secret: Secret) -> Self {
        Self(secret)
    }

    pub(crate) fn hash(&self) -> TokenHash {
        TokenHash::of(self.0.expose().as_bytes())
    }

    pub(crate) fn expose(&self) -> &str {
        self.0.expose()
    }
}

/// base64url без выравнивания (RFC 4648, раздел 5). Собственный кодировщик: ради одной
/// строки токена зависимость от `base64` в composition root не нужна.
fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    // В `buffer` остаются только ещё не выданные биты.
    let (mut buffer, mut pending) = (0_u32, 0_u32);
    for byte in bytes {
        buffer = (buffer << BITS_PER_BYTE) | u32::from(*byte);
        pending += BITS_PER_BYTE;
        while pending >= BITS_PER_SYMBOL {
            pending -= BITS_PER_SYMBOL;
            out.push(symbol(buffer >> pending));
        }
        buffer &= (1 << pending) - 1;
    }
    if pending > 0 {
        // Неполный хвост дополняется нулями справа до целого символа.
        out.push(symbol(buffer << (BITS_PER_SYMBOL - pending)));
    }
    out
}

fn symbol(bits: u32) -> char {
    char::from(ALPHABET[(bits & SYMBOL_MASK) as usize])
}

#[cfg(test)]
#[path = "tests/token.rs"]
mod tests;
