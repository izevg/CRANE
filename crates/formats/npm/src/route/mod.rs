//! Маршрутизация запросов npm-клиентов.

mod tarball;

use crate::ecosystem::parse_name;
use crane_domain::registry::{NameError, PackageName};

pub use tarball::{tarball_file, tarball_version};

const TARBALL_MARK: &str = "-";
const REV_MARK: &str = "-rev";
const TARBALL_SUFFIX: &str = ".tgz";

/// Разобранный запрос. `rev` в путях игнорируется: PUT ничего не разрушает (см. `write`).
/// `tag` не проверен: его проверяет [`crate::parse_tag`]. HEAD сводится к GET на стороне HTTP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    Packument {
        package: PackageName,
    },
    Tarball {
        package: PackageName,
        file: String,
    },
    Write {
        package: PackageName,
    },
    DeleteTarball {
        package: PackageName,
        file: String,
    },
    DeletePackage {
        package: PackageName,
    },
    DistTags {
        package: PackageName,
    },
    SetDistTag {
        package: PackageName,
        tag: String,
    },
    RemoveDistTag {
        package: PackageName,
        tag: String,
    },
    Whoami,
    Ping,
    /// Не проксируется: HTTP отвечает 501.
    Audit,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RouteError {
    #[error("маршрут не найден")]
    NotFound,
    #[error("некорректное имя пакета")]
    Name(#[source] NameError),
}

impl Route {
    /// `path` — уже декодированный остаток после `/npm/<repo>/` (`@scope%2fname` стал `@scope/name`).
    pub fn parse(method: &str, path: &str) -> Result<Self, RouteError> {
        let segments: Vec<&str> = path.split('/').collect();
        if segments
            .iter()
            .any(|s| s.is_empty() || matches!(*s, "." | ".."))
        {
            return Err(RouteError::NotFound);
        }
        match segments.as_slice() {
            [TARBALL_MARK, rest @ ..] => service(method, rest),
            all => package_route(method, all),
        }
    }
}

fn package(name: &str) -> Result<PackageName, RouteError> {
    parse_name(name).map_err(RouteError::Name)
}

fn is_tarball(file: &str) -> bool {
    file.len() > TARBALL_SUFFIX.len() && file.ends_with(TARBALL_SUFFIX)
}

/// Имя пакета из начала сегментов (`name` или `@scope/name`) и остаток.
fn split_package<'s, 'a>(segments: &'s [&'a str]) -> Option<(String, &'s [&'a str])> {
    match segments {
        [scope, name, rest @ ..] if scope.starts_with('@') => {
            Some((format!("{scope}/{name}"), rest))
        }
        [name, rest @ ..] => Some(((*name).to_owned(), rest)),
        [] => None,
    }
}

fn package_route(method: &str, segments: &[&str]) -> Result<Route, RouteError> {
    let (name, rest) = split_package(segments).ok_or(RouteError::NotFound)?;
    match (method, rest) {
        ("GET", []) => Ok(Route::Packument {
            package: package(&name)?,
        }),
        ("GET", [TARBALL_MARK, file]) if is_tarball(file) => Ok(Route::Tarball {
            package: package(&name)?,
            file: (*file).to_owned(),
        }),
        ("PUT", [] | [REV_MARK, _]) => Ok(Route::Write {
            package: package(&name)?,
        }),
        ("DELETE", [TARBALL_MARK, file, REV_MARK, _]) if is_tarball(file) => {
            Ok(Route::DeleteTarball {
                package: package(&name)?,
                file: (*file).to_owned(),
            })
        }
        ("DELETE", [REV_MARK, _]) => Ok(Route::DeletePackage {
            package: package(&name)?,
        }),
        _ => Err(RouteError::NotFound),
    }
}

/// Пути под `-/`.
fn service(method: &str, rest: &[&str]) -> Result<Route, RouteError> {
    match (method, rest) {
        ("GET", ["whoami"]) => Ok(Route::Whoami),
        ("GET", ["ping"]) => Ok(Route::Ping),
        ("POST", ["npm", "v1", "security", _, ..]) => Ok(Route::Audit),
        (_, ["package", tail @ ..]) => dist_tags(method, tail),
        _ => Err(RouteError::NotFound),
    }
}

fn dist_tags(method: &str, tail: &[&str]) -> Result<Route, RouteError> {
    let (name, rest) = split_package(tail).ok_or(RouteError::NotFound)?;
    match (method, rest) {
        ("GET", ["dist-tags"]) => Ok(Route::DistTags {
            package: package(&name)?,
        }),
        ("PUT", ["dist-tags", tag]) => Ok(Route::SetDistTag {
            package: package(&name)?,
            tag: (*tag).to_owned(),
        }),
        ("DELETE", ["dist-tags", tag]) => Ok(Route::RemoveDistTag {
            package: package(&name)?,
            tag: (*tag).to_owned(),
        }),
        _ => Err(RouteError::NotFound),
    }
}
