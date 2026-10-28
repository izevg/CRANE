use crane_domain::proxy::PathError;
use crane_domain::registry::{Ecosystem, NameError, PackageName};

use crate::ecosystem::MAVEN;
use crate::layout::{Algorithm, Resource, Route, RouteError};

fn package(raw: &str) -> PackageName {
    MAVEN.parse_name(raw).unwrap()
}

fn file(raw: &str, dir: &str, file: &str, checksum: Option<Algorithm>) -> Route {
    Route {
        resource: Resource::File {
            package: package(raw),
            dir: dir.to_owned(),
            file: file.to_owned(),
        },
        checksum,
    }
}

fn versions(raw: &str, checksum: Option<Algorithm>) -> Route {
    Route {
        resource: Resource::Versions {
            package: package(raw),
        },
        checksum,
    }
}

fn builds(raw: &str, base: &str, checksum: Option<Algorithm>) -> Route {
    Route {
        resource: Resource::Builds {
            package: package(raw),
            base: base.to_owned(),
        },
        checksum,
    }
}

const SNAPSHOT_JAR: &str = "com/acme/core/1.0-SNAPSHOT/core-1.0-20261028.123456-3-sources.jar.asc";

#[test]
fn layout_paths_are_parsed_into_resources() {
    // Arrange
    let cases = [
        (
            "com/acme/core/1.0/core-1.0.jar",
            file("com.acme:core", "1.0", "core-1.0.jar", None),
        ),
        ("a/b/1.0/b-1.0.pom", file("a:b", "1.0", "b-1.0.pom", None)),
        (
            SNAPSHOT_JAR,
            file(
                "com.acme:core",
                "1.0-SNAPSHOT",
                "core-1.0-20261028.123456-3-sources.jar.asc",
                None,
            ),
        ),
        (
            "com/acme/core/maven-metadata.xml",
            versions("com.acme:core", None),
        ),
        (
            "com/acme/core/1.0-SNAPSHOT/maven-metadata.xml",
            builds("com.acme:core", "1.0", None),
        ),
        (
            "org/apache/maven/plugins/maven-metadata.xml",
            versions("org.apache.maven:plugins", None),
        ),
    ];

    // Act
    let actual = cases.each_ref().map(|(path, _)| Route::parse(path));

    // Assert
    assert_eq!(actual, cases.map(|(_, route)| Ok(route)));
}

#[test]
fn checksum_suffix_selects_the_algorithm_and_keeps_the_resource() {
    // Arrange
    let cases = [
        ("md5", Algorithm::Md5),
        ("sha1", Algorithm::Sha1),
        ("sha256", Algorithm::Sha256),
        ("sha512", Algorithm::Sha512),
    ];

    // Act
    let actual = cases.map(|(suffix, _)| {
        [
            format!("com/acme/core/1.0/core-1.0.jar.{suffix}"),
            format!("com/acme/core/maven-metadata.xml.{suffix}"),
            format!("com/acme/core/1.0-SNAPSHOT/maven-metadata.xml.{suffix}"),
        ]
        .map(|path| Route::parse(&path))
    });

    // Assert
    assert_eq!(
        actual,
        cases.map(|(_, algorithm)| [
            Ok(file(
                "com.acme:core",
                "1.0",
                "core-1.0.jar",
                Some(algorithm)
            )),
            Ok(versions("com.acme:core", Some(algorithm))),
            Ok(builds("com.acme:core", "1.0", Some(algorithm))),
        ])
    );
}

#[test]
fn unsafe_paths_are_rejected_before_anything_else() {
    // Arrange
    let paths = [
        "",
        "com/../core/1.0/core-1.0.jar",
        "../com/acme/core/maven-metadata.xml",
        "com/acme/core/./maven-metadata.xml",
        "/com/acme/core/maven-metadata.xml",
        "com//acme/core/maven-metadata.xml",
        "com/acme/core/1.0/core-1.0.jar/",
        "com/acme\\core/1.0/core-1.0.jar",
        "com/%2e%2e/core/1.0/core-1.0.jar",
        "com/%2E%2E/core/1.0/core-1.0.jar",
        "com/acme%2fevil/core/1.0/core-1.0.jar",
        "com/acme%2Fevil/core/1.0/core-1.0.jar",
        "com/acme%5cevil/core/1.0/core-1.0.jar",
        "com/acme/core/1.0/core-1.0.jar\u{0}",
        "com/acme/core/1.0/core\n-1.0.jar",
        "com/acme/core/1.0/%2e%2e",
        "com/acme/core/1.0/..",
    ];

    // Act
    let actual = paths.map(Route::parse);

    // Assert
    assert_eq!(actual, paths.map(|_| Err(RouteError::UnsafePath)));
}

#[test]
fn paths_off_the_layout_are_rejected() {
    // Arrange
    let paths = [
        "maven-metadata.xml",
        "core/maven-metadata.xml",
        "com/acme/core.jar",
        "com/core/core-1.0.jar",
        "core-SNAPSHOT/1.0-SNAPSHOT/maven-metadata.xml",
        "com/acme/core/1.0/.sha1",
        "com.acme/core/1.0/core-1.0.jar",
        "com.acme/core/maven-metadata.xml",
    ];

    // Act
    let actual = paths.map(Route::parse);

    // Assert
    assert_eq!(actual, paths.map(|_| Err(RouteError::NotLayout)));
}

#[test]
fn invalid_coordinates_report_the_name_error() {
    // Arrange
    let cases = [
        (
            "com/acme/bad:name/1.0/bad:name-1.0.jar",
            NameError::InvalidChar(':'),
        ),
        (
            "com/ac me/core/1.0/core-1.0.jar",
            NameError::InvalidChar(' '),
        ),
        (
            "com/acme/core-SNAPSHOT/1.0/core-SNAPSHOT-1.0.jar",
            NameError::Reserved("core-SNAPSHOT".to_owned()),
        ),
        (
            "com/acme/core/-SNAPSHOT/maven-metadata.xml",
            NameError::Reserved("-SNAPSHOT".to_owned()),
        ),
    ];

    // Act
    let actual = cases.each_ref().map(|(path, _)| Route::parse(path));

    // Assert
    assert_eq!(actual, cases.map(|(_, error)| Err(RouteError::Name(error))));
}

#[test]
fn upstream_path_is_rebuilt_from_the_parsed_route() {
    // Arrange
    let paths = [
        "com/acme/core/1.0/core-1.0.jar",
        "com/acme/core/1.0/core-1.0.jar.sha1",
        SNAPSHOT_JAR,
        "com/acme/core/maven-metadata.xml",
        "com/acme/core/maven-metadata.xml.md5",
        "com/acme/core/1.0-SNAPSHOT/maven-metadata.xml",
        "com/acme/core/1.0-SNAPSHOT/maven-metadata.xml.sha512",
        "a/b/1.0/b-1.0.pom.sha256",
    ];

    // Act
    let actual = paths.map(|path| {
        Route::parse(path)
            .ok()
            .and_then(|route| route.upstream_path().ok())
            .map(|upstream| upstream.as_str().to_owned())
    });

    // Assert
    assert_eq!(actual, paths.map(|path| Some(path.to_owned())));
}

#[test]
fn upstream_path_refuses_what_must_not_leave_the_process() {
    // Arrange
    let cases = [
        (
            "com/acme/core/1.0/core-1.0:x.jar",
            PathError::ForbiddenChar(':'),
        ),
        (
            "com/acme/core/1.0/core 1.0.jar",
            PathError::InvalidByte(b' '),
        ),
    ];

    // Act
    let actual = cases.each_ref().map(|(path, _)| {
        Route::parse(path)
            .map_err(|_| PathError::Empty)
            .and_then(|route| route.upstream_path())
            .map(|upstream| upstream.as_str().to_owned())
    });

    // Assert
    assert_eq!(actual, cases.map(|(_, error)| Err(error)));
}
