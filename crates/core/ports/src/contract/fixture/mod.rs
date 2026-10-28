//! Общие данные сценариев: имена, версии и упрощённая экосистема с правилами cargo.
//!
//! `ports` не зависит от `formats`, поэтому настоящую грамматику сюда не подтянуть: сценариям
//! каталога нужны только согласованные нормализованные формы и кандидаты namespace.

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespacePattern, NamespaceShape, OrderKeyBuilder, PackageName,
    Version, VersionError,
};

/// Нормализация и разделители cargo без проверки грамматики.
struct Underscored;

impl Ecosystem for Underscored {
    fn format(&self) -> Format {
        Format::Cargo
    }

    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        Ok(PackageName::from_parts(self.normalize(raw), raw.to_owned()))
    }

    fn parse_version(&self, _raw: &str) -> Result<Version, VersionError> {
        Err(VersionError::Unsupported(Format::Cargo))
    }

    fn normalize(&self, raw: &str) -> String {
        raw.to_ascii_lowercase().replace('-', "_")
    }

    fn separator(&self) -> char {
        '_'
    }

    fn accepted_separators(&self) -> &'static [char] {
        &['-', '_']
    }

    fn namespace_shape(&self) -> NamespaceShape {
        NamespaceShape::Delimited
    }
}

pub(super) fn ecosystem() -> &'static dyn Ecosystem {
    &Underscored
}

pub(super) fn name(raw: &str) -> PackageName {
    ecosystem()
        .parse_name(raw)
        .expect("тестовое имя пакета валидно")
}

/// Версия `a.b.c[+build]`: build metadata не входит в каноническую форму, как у `SemVer`.
pub(super) fn version(raw: &str) -> Version {
    let core = raw.split('+').next().unwrap_or(raw);
    let order = core
        .split('.')
        .fold(OrderKeyBuilder::new(), |builder, part| {
            builder.number(part.parse().expect("тестовая версия — числа через точку"))
        })
        .finish();
    Version::from_parts(core.to_owned(), raw.to_owned(), order, false)
}

/// Кандидаты namespace для имени, от самого специфичного.
pub(super) fn candidates(name: &PackageName) -> Vec<NamespacePattern> {
    NamespacePattern::covering(ecosystem(), name)
}

pub(super) fn pattern(raw: &str) -> NamespacePattern {
    NamespacePattern::parse(ecosystem(), raw).expect("тестовый паттерн валиден")
}
