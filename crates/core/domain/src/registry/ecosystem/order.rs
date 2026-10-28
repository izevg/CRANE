//! Порядко-сохраняющее кодирование: байтовое сравнение ключей равно сравнению
//! последовательностей элементов, из которых они собраны.

/// Ключ порядка версии; сравнивается побайтно.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct OrderKey(Vec<u8>);

impl OrderKey {
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Ключ из сохранённых байтов (каталог). Содержимое не проверяется: его собрал
    /// `OrderKeyBuilder` при публикации.
    #[must_use]
    pub const fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

/// Строит ключ из последовательности элементов. Одна последовательность — одна версия,
/// элементы на одной позиции у всех версий экосистемы должны быть одного вида.
#[derive(Debug, Default)]
pub struct OrderKeyBuilder(Vec<u8>);

/// Байт после `0x00` внутри текста, обозначающий сам нулевой байт.
const ESCAPED_NUL: u8 = 0xFF;
/// Байт после `0x00`, обозначающий конец текста; меньше `ESCAPED_NUL`, поэтому префикс
/// меньше своего продолжения, в том числе продолжения нулевым байтом.
const TEXT_END: u8 = 0x01;

impl OrderKeyBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    /// Число: байт длины и big-endian без ведущих нулей. Длиннее значит больше, поэтому 9 < 10.
    #[must_use]
    pub fn number(mut self, value: u64) -> Self {
        let digits: Vec<u8> = value
            .to_be_bytes()
            .into_iter()
            .skip_while(|byte| *byte == 0)
            .collect();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "у u64 не больше восьми значащих байт"
        )]
        self.0.push(digits.len() as u8);
        self.0.extend_from_slice(&digits);
        self
    }

    /// Текст: нулевой байт экранируется, в конце терминатор. Префикс меньше продолжения.
    #[must_use]
    pub fn text(mut self, value: &str) -> Self {
        for &byte in value.as_bytes() {
            self.0.push(byte);
            if byte == 0 {
                self.0.push(ESCAPED_NUL);
            }
        }
        self.0.extend_from_slice(&[0, TEXT_END]);
        self
    }

    /// Ранг варианта (например, pre-release < release): меньший ранг даёт меньший ключ.
    #[must_use]
    pub fn rank(mut self, value: u8) -> Self {
        self.0.push(value);
        self
    }

    #[must_use]
    pub fn finish(self) -> OrderKey {
        OrderKey(self.0)
    }
}

#[cfg(test)]
#[path = "tests/order.rs"]
mod tests;
