/// 16 байт в base64 без паддинга занимают 22 символа.
const SYMBOLS: usize = 22;
const BITS_PER_SYMBOL: u32 = 6;

/// 16 байт из urlsafe base64 (клиенты шлют `md5_digest` в нём и не в hex); хвостовые `=` допускаются.
/// Своя функция дешевле зависимости ради одного поля формы.
pub(super) fn decode_md5(text: &str) -> Option<[u8; 16]> {
    let text = text.trim_end_matches('=');
    if text.len() != SYMBOLS {
        return None;
    }
    let mut bytes = [0_u8; 16];
    let (mut written, mut accumulated, mut bits) = (0, 0_u32, 0);
    for symbol in text.bytes() {
        let value = match symbol {
            b'A'..=b'Z' => symbol - b'A',
            b'a'..=b'z' => symbol - b'a' + 26,
            b'0'..=b'9' => symbol - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        };
        accumulated = accumulated << BITS_PER_SYMBOL | u32::from(value);
        bits += BITS_PER_SYMBOL;
        if bits >= 8 {
            bits -= 8;
            *bytes.get_mut(written)? = u8::try_from(accumulated >> bits & 0xFF).ok()?;
            accumulated &= (1 << bits) - 1;
            written += 1;
        }
    }
    Some(bytes)
}
