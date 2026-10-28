//! Выбор реализации `Ecosystem` по тегу формата: единственное место, где `entry` знает все
//! format-крейты, чтобы `app` и адаптеры хранилищ грамматик не знали.

use crane_domain::registry::{Ecosystem, Format};
use crane_format_cargo::CARGO;
use crane_format_maven::MAVEN;
use crane_format_npm::NPM;
use crane_format_nuget::NUGET;
use crane_format_pypi::PYPI;

/// Экосистема формата. Исчерпывающий `match`: новый формат не соберётся, пока его не добавят сюда.
pub(crate) fn ecosystem(format: Format) -> &'static dyn Ecosystem {
    match format {
        Format::Cargo => &CARGO,
        Format::Npm => &NPM,
        Format::PyPi => &PYPI,
        Format::Maven => &MAVEN,
        Format::NuGet => &NUGET,
    }
}

#[cfg(test)]
mod tests;
