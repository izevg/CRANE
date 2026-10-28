//! Тестовая экосистема с правилами cargo: `app` не зависит от `formats`, поэтому настоящую
//! грамматику сюда не подтянуть, а сценариям нужны только согласованные формы имён и версий.

use crane_domain::registry::{
    Ecosystem, Format, NameError, NamespacePattern, NamespaceShape, OrderKeyBuilder, PackageName,
    Version, VersionError,
};

pub(crate) struct TestCargo;

impl Ecosystem for TestCargo {
    fn format(&self) -> Format {
        Format::Cargo
    }

    fn parse_name(&self, raw: &str) -> Result<PackageName, NameError> {
        Ok(PackageName::from_parts(normalize(raw), raw.to_owned()))
    }

    fn parse_version(&self, raw: &str) -> Result<Version, VersionError> {
        Ok(version_of(raw))
    }

    fn prefix_base(&self, raw: &str) -> Result<String, NameError> {
        Ok(normalize(raw))
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

/// Выбор экосистемы для `Registry::new`: во всех сценариях правила cargo.
fn normalize(raw: &str) -> String {
    raw.to_ascii_lowercase().replace('-', "_")
}

pub(crate) fn ecosystems(_format: Format) -> &'static dyn Ecosystem {
    &TestCargo
}

/// Версия `a.b.c[+build]`: build metadata не входит в каноническую форму, как у `SemVer`.
fn version_of(raw: &str) -> Version {
    let core = raw.split('+').next().unwrap_or(raw);
    let order = core
        .split('.')
        .fold(OrderKeyBuilder::new(), |builder, part| {
            builder.number(part.parse().unwrap())
        })
        .finish();
    Version::from_parts(core.to_owned(), raw.to_owned(), order, false)
}

pub(crate) fn pattern(raw: &str) -> NamespacePattern {
    NamespacePattern::parse(&TestCargo, raw).unwrap()
}
