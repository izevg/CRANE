//! Порядок `GenericVersionScheme` Maven Resolver 1.9.x в виде побайтно сравниваемого ключа.
//!
//! Источник истины — `GenericVersion.java`, ветка `maven-resolver-1.9.x`, последний коммит файла
//! `24dd0e04139bbdd440472f14de8da2ee2b8cf2e8` (MRESOLVER-686, ветка по состоянию на
//! `bc9cada2a125db1f6f6cba3eeedc87b2fce73e1b`):
//! <https://github.com/apache/maven-resolver/blob/maven-resolver-1.9.x/maven-resolver-util/src/main/java/org/eclipse/aether/util/version/GenericVersion.java>
//!
//! Токенизация и `trimPadding` перенесены как есть. Сравнение `compareTo` в ключ переписано так:
//! после `trimPadding` исход сравнения в позиции `i > 0` определяют только три факта о хвосте,
//! и каждый кодируется байтом класса перед элементом (см. [`class_of`]). Что ключ совпадает
//! с `compareTo`, доказывают тесты против его прямого порта.

use std::cmp::Ordering;

use crane_domain::registry::OrderKeyBuilder;

/// Ранги видов элемента повторяют `Item.KIND_*` в Java: при несовпадении вида в позиции 0
/// Java сравнивает именно их.
const KIND_MIN: u8 = 0;
const KIND_QUALIFIER: u8 = 2;
const KIND_TEXT: u8 = 3;
const KIND_NUMBER: u8 = 4;
const KIND_MAX: u8 = 8;

/// Классы хвоста в позиции `i > 0`, по возрастанию.
///
/// «Продолжает» — первый элемент хвоста того же рода (число/не число), что предыдущий;
/// «переключает» — другого. Хвост, продолжающий род, сравнивается с хвостом, который род
/// переключил, по первому ненулевому элементу своего рода (Java `comparePadding` с фильтром),
/// остальное у переключившего игнорируется. Конец хвоста («паддинг») сравнивается с хвостом
/// по его первому ненулевому элементу любого рода.
const CLASS_CONTINUES_BELOW: u8 = 1;
const CLASS_SWITCHES_BELOW: u8 = 2;
const CLASS_END: u8 = 3;
/// Timestamp-сборка `X-SNAPSHOT`: сразу над концом ключа `X-SNAPSHOT`, но под любым продолжением
/// (`X-SNAPSHOT.1`, `X-SNAPSHOT-sp`). Ниже релиза `X` она по построению: класс решается раньше.
pub(super) const CLASS_BUILD: u8 = 4;
const CLASS_SWITCHES_ABOVE: u8 = 5;
const CLASS_CONTINUES_ABOVE: u8 = 6;

/// Смещение, переводящее значение квалификатора `-5..=1` в ранг `0..=6`.
const QUALIFIER_OFFSET: i8 = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Item {
    Min,
    /// Значение известного квалификатора (`alpha` = -5 … `sp` = 1).
    Qualifier(i8),
    /// Неизвестная строка в нижнем регистре.
    Text(String),
    /// Десятичная запись без ведущих нулей (`"0"` для нуля): произвольная длина, как `BigInteger`.
    Number(String),
    Max,
}

impl Item {
    /// `Item.isNumber()`: `min` и `max` в Java тоже «числа».
    const fn is_number(&self) -> bool {
        matches!(self, Self::Min | Self::Number(_) | Self::Max)
    }

    /// `Item.compareTo(null)`: сравнение с паддингом (`0` или `ga`).
    fn against_padding(&self) -> Ordering {
        match self {
            Self::Min => Ordering::Less,
            Self::Qualifier(value) => value.cmp(&0),
            Self::Number(digits) if digits == "0" => Ordering::Equal,
            Self::Max | Self::Text(_) | Self::Number(_) => Ordering::Greater,
        }
    }
}

/// Токенизатор `GenericVersion.Tokenizer` и `trimPadding`. Вход — ASCII без разделителей пути.
pub(super) fn items(version: &str) -> Vec<Item> {
    let mut items = Vec::new();
    let mut index = 0;
    while let Some(item) = next_item(version, &mut index) {
        items.push(item);
    }
    trim_padding(&mut items);
    items
}

fn next_item(version: &str, index: &mut usize) -> Option<Item> {
    let bytes = version.as_bytes();
    if *index >= bytes.len() {
        return None;
    }
    // -2 — ничего не прочитано, -1 — буквы, 0 — только нули, 1 — есть ненулевая цифра.
    let mut state: i8 = -2;
    let mut start = *index;
    let mut end = bytes.len();
    let mut terminated_by_number = false;
    while *index < bytes.len() {
        let byte = bytes[*index];
        if matches!(byte, b'.' | b'-' | b'_') {
            end = *index;
            *index += 1;
            break;
        }
        if byte.is_ascii_digit() {
            if state == -1 {
                end = *index;
                terminated_by_number = true;
                break;
            }
            if state == 0 {
                // Ведущие нули не значимы: сдвигаем начало токена.
                start += 1;
            }
            state = i8::from(state > 0 || byte != b'0');
        } else {
            if state >= 0 {
                end = *index;
                break;
            }
            state = -1;
        }
        *index += 1;
    }
    // Пустой токен между разделителями — нуль.
    let token = if end > start {
        &version[start..end]
    } else {
        "0"
    };
    let is_number = end <= start || state >= 0;
    let is_last = *index >= bytes.len();
    Some(if is_number {
        Item::Number(token.to_owned())
    } else {
        text_item(token, is_last, terminated_by_number)
    })
}

/// Нечисловой токен: `min`/`max` только последним, `a`/`b`/`m` только перед цифрой.
fn text_item(token: &str, is_last: bool, terminated_by_number: bool) -> Item {
    let lower = token.to_ascii_lowercase();
    if is_last {
        match lower.as_str() {
            "min" => return Item::Min,
            "max" => return Item::Max,
            _ => {}
        }
    }
    if terminated_by_number {
        match lower.as_str() {
            "a" => return Item::Qualifier(-5),
            "b" => return Item::Qualifier(-4),
            "m" => return Item::Qualifier(-3),
            _ => {}
        }
    }
    match qualifier(&lower) {
        Some(value) => Item::Qualifier(value),
        None => Item::Text(lower),
    }
}

fn qualifier(lower: &str) -> Option<i8> {
    Some(match lower {
        "alpha" => -5,
        "beta" => -4,
        "milestone" => -3,
        "cr" | "rc" => -2,
        "snapshot" => -1,
        "ga" | "final" | "release" => 0,
        "sp" => 1,
        _ => return None,
    })
}

/// `GenericVersion.trimPadding`: убирает нулевые элементы в конце каждой серии одного рода
/// (число/не число), кроме первого элемента серии, если за ним что-то следует.
fn trim_padding(items: &mut Vec<Item>) {
    let mut kind: Option<bool> = None;
    let mut end = items.len().saturating_sub(1);
    let mut i = end;
    while i > 0 {
        let is_number = items[i].is_number();
        if kind != Some(is_number) {
            end = i;
            kind = Some(is_number);
        }
        let last_of_series = end == i;
        let not_series_start = i == items.len() - 1 || items[i - 1].is_number() == is_number;
        if last_of_series && not_series_start && items[i].against_padding() == Ordering::Equal {
            items.remove(i);
            end -= 1;
        }
        i -= 1;
    }
}

/// Ключ списка элементов без завершения: сборка дописывает свой хвост, остальные — `CLASS_END`.
pub(super) fn key_prefix(items: &[Item]) -> OrderKeyBuilder {
    items
        .iter()
        .enumerate()
        .fold(OrderKeyBuilder::new(), |builder, (index, item)| {
            // В позиции 0 Java сравнивает виды элементов напрямую: класс не нужен.
            let builder = if index == 0 {
                builder
            } else {
                builder.rank(class_of(items, index))
            };
            encode(builder, item)
        })
}

/// Класс хвоста `items[index..]` относительно предыдущего элемента.
///
/// После `trimPadding` последний элемент серии длиннее одного ненулевой, поэтому знак по
/// продолжающей серии и по всему хвосту никогда не равен нулю. Нулевой знак тем не менее
/// отнесён к «выше»: ключ остаётся полным порядком, а не паникой.
fn class_of(items: &[Item], index: usize) -> u8 {
    let previous_is_number = items[index - 1].is_number();
    let tail = &items[index..];
    if items[index].is_number() == previous_is_number {
        let sign = first_nonzero(tail.iter().filter(|i| i.is_number() == previous_is_number));
        match sign {
            Ordering::Less => CLASS_CONTINUES_BELOW,
            _ => CLASS_CONTINUES_ABOVE,
        }
    } else {
        match first_nonzero(tail.iter()) {
            Ordering::Less => CLASS_SWITCHES_BELOW,
            _ => CLASS_SWITCHES_ABOVE,
        }
    }
}

/// Знак первого элемента, отличного от паддинга; `Equal`, если таких нет.
fn first_nonzero<'a>(mut tail: impl Iterator<Item = &'a Item>) -> Ordering {
    tail.find_map(|item| Some(item.against_padding()).filter(|sign| sign.is_ne()))
        .unwrap_or(Ordering::Equal)
}

fn encode(builder: OrderKeyBuilder, item: &Item) -> OrderKeyBuilder {
    match item {
        Item::Min => builder.rank(KIND_MIN),
        Item::Max => builder.rank(KIND_MAX),
        Item::Number(digits) => builder.rank(KIND_NUMBER).digits(digits),
        Item::Text(text) => builder.rank(KIND_TEXT).text(text),
        Item::Qualifier(value) => builder
            .rank(KIND_QUALIFIER)
            // Значения -5..=1 помещаются в `u8` после сдвига.
            .rank(u8::try_from(value + QUALIFIER_OFFSET).unwrap_or(u8::MAX)),
    }
}

/// Конец ключа обычной версии.
pub(super) fn terminate(builder: OrderKeyBuilder) -> OrderKeyBuilder {
    builder.rank(CLASS_END)
}
