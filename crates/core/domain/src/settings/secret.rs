use zeroize::Zeroizing;

/// Секрет (токен, URL базы, ключ доступа). Без `Debug`, `Display`, `Clone` и `PartialEq`:
/// случайно попасть в журнал, быть размноженным или сравненным не в постоянное время он не может.
/// Память обнуляется при `Drop`: значение не переживает секрет в освобождённой куче.
#[allow(
    missing_debug_implementations,
    reason = "Debug секрета запрещён: значение не должно попасть в журнал"
)]
pub struct Secret(Zeroizing<String>);

impl Secret {
    #[must_use]
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    /// Единственный способ прочитать значение: вызов виден в ревью и при поиске по коду.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }
}
