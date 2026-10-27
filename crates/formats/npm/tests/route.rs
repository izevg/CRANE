#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "в тестах падение unwrap — это падение теста"
)]

use crane_domain::registry::{Format, NameError, PackageName};
use crane_format_npm::{Route, RouteError, tarball_file, tarball_version};

fn pkg(raw: &str) -> PackageName {
    PackageName::parse(Format::Npm, raw).unwrap()
}

#[test]
fn route_parse_maps_package_paths_for_every_verb() {
    // Arrange
    let cases = [
        ("GET", "left-pad"),
        ("GET", "@babel/core"),
        ("GET", "@babel/core/-/core-7.0.0.tgz"),
        ("GET", "left-pad/-/left-pad-1.0.0.tgz"),
        ("PUT", "left-pad"),
        ("PUT", "@babel/core/-rev/3-abc"),
        ("DELETE", "left-pad/-/left-pad-1.0.0.tgz/-rev/3-abc"),
        ("DELETE", "left-pad/-rev/3-abc"),
        ("DELETE", "@babel/core/-/core-7.0.0.tgz/-rev/1-x"),
        ("DELETE", "@babel/core/-rev/1-x"),
    ];

    // Act
    let actual = cases.map(|(method, path)| Route::parse(method, path));

    // Assert
    assert_eq!(
        actual,
        [
            Ok(Route::Packument {
                package: pkg("left-pad")
            }),
            Ok(Route::Packument {
                package: pkg("@babel/core")
            }),
            Ok(Route::Tarball {
                package: pkg("@babel/core"),
                file: "core-7.0.0.tgz".into()
            }),
            Ok(Route::Tarball {
                package: pkg("left-pad"),
                file: "left-pad-1.0.0.tgz".into()
            }),
            Ok(Route::Write {
                package: pkg("left-pad")
            }),
            Ok(Route::Write {
                package: pkg("@babel/core")
            }),
            Ok(Route::DeleteTarball {
                package: pkg("left-pad"),
                file: "left-pad-1.0.0.tgz".into()
            }),
            Ok(Route::DeletePackage {
                package: pkg("left-pad")
            }),
            Ok(Route::DeleteTarball {
                package: pkg("@babel/core"),
                file: "core-7.0.0.tgz".into()
            }),
            Ok(Route::DeletePackage {
                package: pkg("@babel/core")
            }),
        ]
    );
}

#[test]
fn route_parse_maps_service_paths() {
    // Arrange
    let cases = [
        ("GET", "-/whoami"),
        ("GET", "-/ping"),
        ("POST", "-/npm/v1/security/advisories/bulk"),
        ("POST", "-/npm/v1/security/audits/quick"),
        ("GET", "-/package/left-pad/dist-tags"),
        ("GET", "-/package/@babel/core/dist-tags"),
        ("PUT", "-/package/@babel/core/dist-tags/next"),
        ("DELETE", "-/package/left-pad/dist-tags/next"),
    ];

    // Act
    let actual = cases.map(|(method, path)| Route::parse(method, path));

    // Assert
    assert_eq!(
        actual,
        [
            Ok(Route::Whoami),
            Ok(Route::Ping),
            Ok(Route::Audit),
            Ok(Route::Audit),
            Ok(Route::DistTags {
                package: pkg("left-pad")
            }),
            Ok(Route::DistTags {
                package: pkg("@babel/core")
            }),
            Ok(Route::SetDistTag {
                package: pkg("@babel/core"),
                tag: "next".into()
            }),
            Ok(Route::RemoveDistTag {
                package: pkg("left-pad"),
                tag: "next".into()
            }),
        ]
    );
}

#[test]
fn route_parse_refuses_everything_else_as_not_found() {
    // Arrange
    let cases = [
        ("GET", ""),
        ("GET", "/left-pad"),
        ("GET", "left-pad/"),
        ("GET", "left-pad//x"),
        ("GET", ".."),
        ("GET", "left-pad/../x"),
        ("GET", "./left-pad"),
        ("GET", "left-pad/-/readme.txt"),
        ("GET", "left-pad/-/.tgz"),
        ("GET", "left-pad/extra"),
        ("PATCH", "left-pad"),
        ("DELETE", "left-pad"),
        ("POST", "left-pad"),
        ("DELETE", "left-pad/-/left-pad-1.0.0.tgz"),
        ("POST", "-/npm/v1/security"),
        ("GET", "-/npm/v1/security/audits"),
        ("GET", "-/unknown"),
        ("GET", "-"),
        ("GET", "-/package/left-pad"),
        ("POST", "-/package/left-pad/dist-tags"),
        ("PUT", "-/package/left-pad/dist-tags"),
        ("GET", "@babel/core/-/x"),
    ];

    // Act
    let actual = cases.map(|(method, path)| Route::parse(method, path));

    // Assert
    assert_eq!(actual, cases.map(|_| Err(RouteError::NotFound)));
}

#[test]
fn route_parse_reports_bad_name_in_matching_route() {
    // Arrange
    let cases = [
        ("GET", "@scope"),
        ("GET", ".hidden"),
        ("PUT", "_x"),
        ("GET", "node_modules"),
        ("GET", "@a/.b/-/x.tgz"),
        ("GET", "-/package/@scope/dist-tags"),
        ("PUT", "-/package/.x/dist-tags/next"),
    ];

    // Act
    let actual = cases.map(|(method, path)| Route::parse(method, path).unwrap_err());

    // Assert
    assert_eq!(
        actual,
        [
            RouteError::Name(NameError::MalformedScope),
            RouteError::Name(NameError::InvalidStart('.')),
            RouteError::Name(NameError::InvalidStart('_')),
            RouteError::Name(NameError::Reserved("node_modules".into())),
            RouteError::Name(NameError::InvalidStart('.')),
            RouteError::NotFound,
            RouteError::Name(NameError::InvalidStart('.')),
        ]
    );
}

#[test]
fn tarball_file_uses_basename_of_scoped_name() {
    // Arrange
    let cases = [
        (pkg("left-pad"), "1.0.0"),
        (pkg("@babel/core"), "7.0.0-rc.1+b5"),
    ];

    // Act
    let actual = cases.map(|(package, version)| {
        tarball_file(
            &package,
            &crane_domain::registry::Version::parse(Format::Npm, version).unwrap(),
        )
    });

    // Assert
    assert_eq!(actual, ["left-pad-1.0.0.tgz", "core-7.0.0-rc.1+b5.tgz"]);
}

#[test]
fn tarball_version_is_strict_inverse_of_tarball_file() {
    // Arrange
    let package = pkg("@babel/core");
    let files = [
        "core-7.0.0.tgz",
        "core-7.0.0-rc.1.tgz",
        "core-7.0.tgz",
        "other-7.0.0.tgz",
        "core-7.0.0.tar",
        "core-7.0.0",
        "7.0.0.tgz",
        "core--7.0.0.tgz",
        "core-v7.0.0.tgz",
        "@babel/core-7.0.0.tgz",
    ];

    // Act
    let actual =
        files.map(|file| tarball_version(&package, file).map(|v| v.as_written().to_owned()));

    // Assert
    assert_eq!(
        actual,
        [
            Some("7.0.0".to_owned()),
            Some("7.0.0-rc.1".to_owned()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        ]
    );
}
