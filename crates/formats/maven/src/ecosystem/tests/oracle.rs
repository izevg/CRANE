//! Прямой порт `GenericVersion` (maven-resolver 1.9.x, коммит файла
//! `24dd0e04139bbdd440472f14de8da2ee2b8cf2e8`) как оракул порядка. Намеренно не использует
//! код токенизации и паддинга из `generic.rs`: оракул должен ловить ошибки в нём.
//! Имена и структура повторяют Java, чтобы порт сверялся с исходником построчно.

use std::cmp::Ordering;

const KIND_MAX: i32 = 8;
const KIND_BIGINT: i32 = 5;
const KIND_INT: i32 = 4;
const KIND_STRING: i32 = 3;
const KIND_QUALIFIER: i32 = 2;
const KIND_MIN: i32 = 0;

#[derive(Debug, Clone)]
enum Value {
    None,
    Int(i32),
    Big(String),
    Str(String),
}

#[derive(Debug, Clone)]
struct Item {
    kind: i32,
    value: Value,
}

impl Item {
    fn is_number(&self) -> bool {
        self.kind & KIND_QUALIFIER == 0
    }

    /// `compareTo(null)`: `None` в Java — паддинг (`0` или `ga`).
    fn compare_to(&self, that: Option<&Self>) -> i32 {
        let Some(that) = that else {
            return match (self.kind, &self.value) {
                (KIND_MIN, _) => -1,
                (KIND_MAX | KIND_BIGINT | KIND_STRING, _) => 1,
                (_, Value::Int(value)) => *value,
                _ => 0,
            };
        };
        let rel = self.kind - that.kind;
        if rel != 0 {
            return rel;
        }
        match (&self.value, &that.value) {
            (Value::Big(a), Value::Big(b)) => ordering_sign(a.len().cmp(&b.len()).then(a.cmp(b))),
            (Value::Int(a), Value::Int(b)) => ordering_sign(a.cmp(b)),
            (Value::Str(a), Value::Str(b)) => ordering_sign(a.cmp(b)),
            _ => 0,
        }
    }
}

const fn ordering_sign(ordering: Ordering) -> i32 {
    match ordering {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

struct Tokenizer {
    version: Vec<char>,
    index: usize,
    token: String,
    number: bool,
    terminated_by_number: bool,
}

impl Tokenizer {
    fn new(version: &str) -> Self {
        let version = if version.is_empty() { "0" } else { version };
        Self {
            version: version.chars().collect(),
            index: 0,
            token: String::new(),
            number: false,
            terminated_by_number: false,
        }
    }

    fn next(&mut self) -> bool {
        if self.index >= self.version.len() {
            return false;
        }
        let mut state = -2;
        let mut start = self.index;
        let mut end = self.version.len();
        self.terminated_by_number = false;
        while self.index < self.version.len() {
            let c = self.version[self.index];
            if c == '.' || c == '-' || c == '_' {
                end = self.index;
                self.index += 1;
                break;
            }
            if c.is_ascii_digit() {
                if state == -1 {
                    end = self.index;
                    self.terminated_by_number = true;
                    break;
                }
                if state == 0 {
                    start += 1;
                }
                state = i32::from(state > 0 || c != '0');
            } else {
                if state >= 0 {
                    end = self.index;
                    break;
                }
                state = -1;
            }
            self.index += 1;
        }
        if end > start {
            self.token = self.version[start..end].iter().collect();
            self.number = state >= 0;
        } else {
            self.token = "0".to_owned();
            self.number = true;
        }
        true
    }

    fn to_item(&self) -> Item {
        if self.number {
            return self.number_item();
        }
        let lower = self.token.to_ascii_lowercase();
        if self.index >= self.version.len() {
            if lower == "min" {
                return Item {
                    kind: KIND_MIN,
                    value: Value::None,
                };
            }
            if lower == "max" {
                return Item {
                    kind: KIND_MAX,
                    value: Value::None,
                };
            }
        }
        if self.terminated_by_number && self.token.len() == 1 {
            let single = match lower.as_str() {
                "a" => Some(-5),
                "b" => Some(-4),
                "m" => Some(-3),
                _ => None,
            };
            if let Some(value) = single {
                return Item {
                    kind: KIND_QUALIFIER,
                    value: Value::Int(value),
                };
            }
        }
        match known_qualifier(&lower) {
            Some(value) => Item {
                kind: KIND_QUALIFIER,
                value: Value::Int(value),
            },
            None => Item {
                kind: KIND_STRING,
                value: Value::Str(lower),
            },
        }
    }

    fn number_item(&self) -> Item {
        if self.token.len() < 10 {
            Item {
                kind: KIND_INT,
                value: Value::Int(self.token.parse().unwrap()),
            }
        } else {
            Item {
                kind: KIND_BIGINT,
                value: Value::Big(self.token.clone()),
            }
        }
    }
}

fn known_qualifier(lower: &str) -> Option<i32> {
    match lower {
        "alpha" => Some(-5),
        "beta" => Some(-4),
        "milestone" => Some(-3),
        "cr" | "rc" => Some(-2),
        "snapshot" => Some(-1),
        "ga" | "final" | "release" | "" => Some(0),
        "sp" => Some(1),
        _ => None,
    }
}

fn parse(version: &str) -> Vec<Item> {
    let mut items = Vec::new();
    let mut tokenizer = Tokenizer::new(version);
    while tokenizer.next() {
        items.push(tokenizer.to_item());
    }
    trim_padding(&mut items);
    items
}

fn trim_padding(items: &mut Vec<Item>) {
    let mut number: Option<bool> = None;
    let mut end = items.len().saturating_sub(1);
    // Java перебирает индексы от последнего исходного к первому, а длина списка меняется на ходу.
    for i in (1..items.len()).rev() {
        let item_is_number = items[i].is_number();
        if number != Some(item_is_number) {
            end = i;
            number = Some(item_is_number);
        }
        if end == i
            && (i == items.len() - 1 || items[i - 1].is_number() == item_is_number)
            && items[i].compare_to(None) == 0
        {
            items.remove(i);
            end -= 1;
        }
    }
}

fn compare_padding(items: &[Item], index: usize, number: Option<bool>) -> i32 {
    let mut rel = 0;
    for item in &items[index..] {
        if number.is_some_and(|n| n != item.is_number()) {
            continue;
        }
        rel = item.compare_to(None);
        if rel != 0 {
            break;
        }
    }
    rel
}

/// `GenericVersion.compareTo`: знак сравнения двух версий.
pub(super) fn compare(a: &str, b: &str) -> Ordering {
    let left = parse(a);
    let right = parse(b);
    let mut number = true;
    let mut index = 0;
    let rel = loop {
        if index >= left.len() && index >= right.len() {
            break 0;
        } else if index >= left.len() {
            break -compare_padding(&right, index, None);
        } else if index >= right.len() {
            break compare_padding(&left, index, None);
        }
        let left_item = &left[index];
        let right_item = &right[index];
        if left_item.is_number() == right_item.is_number() {
            let rel = left_item.compare_to(Some(right_item));
            if rel != 0 {
                break rel;
            }
            number = left_item.is_number();
        } else if index == 0 {
            break left_item.compare_to(Some(right_item));
        } else if number == left_item.is_number() {
            break compare_padding(&left, index, Some(number));
        } else {
            break -compare_padding(&right, index, Some(number));
        }
        index += 1;
    };
    rel.cmp(&0)
}
