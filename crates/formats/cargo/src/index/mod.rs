//! Раскладка файлов индекса и NDJSON-строки версий.

mod line;

use crane_domain::registry::{Format, PackageName};

pub(crate) use line::{About, IndexDep, IndexMeta, StoredMeta};
pub use line::{IndexError, render_index};

/// Путь файла индекса для написания имени: нижний регистр, 1/, 2/, 3/{c}/, {ab}/{cd}/.
#[must_use]
pub fn index_path(name: &PackageName) -> String {
    // Каталог считается по написанному имени, а не по нормализованному:
    // cargo строит путь из имени как оно есть, `-` и `_` для него разные символы.
    let lower = name.as_written().to_ascii_lowercase();
    let head =
        |from: usize, to: usize| -> String { lower.chars().skip(from).take(to - from).collect() };
    match lower.len() {
        1 | 2 => format!("{}/{lower}", lower.len()),
        3 => format!("3/{}/{lower}", head(0, 1)),
        _ => format!("{}/{}/{lower}", head(0, 2), head(2, 4)),
    }
}

/// Разбор пути индекса (без ведущего '/').
#[must_use]
pub fn parse_index_path(path: &str) -> Option<PackageName> {
    let file = path.rsplit('/').next()?;
    let name = PackageName::parse(Format::Cargo, file).ok()?;
    // Сверка с канонической раскладкой отклоняет заглавные буквы в пути,
    // чужие каталоги и лишние сегменты разом, без отдельных проверок.
    (index_path(&name) == path).then_some(name)
}
