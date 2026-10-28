use std::time::{Duration, SystemTime};

use crane_format_pypi::{DistFile, Project, Yanked, hosted};

use crate::common::{BASE, asset, digest, pkg, stored};

/// Ожидаемый файл страницы: ссылка строится от нормализованного имени и канонической версии.
fn expected(name: &str, url_version: &str, byte: u8, size: u64, at: u64) -> DistFile {
    DistFile {
        filename: name.to_owned(),
        url: format!("{BASE}/files/acme-core/{url_version}/{name}"),
        sha256: digest(byte),
        requires_python: None,
        yanked: Yanked::No,
        size,
        upload_time: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(at)),
    }
}

#[test]
fn every_asset_of_every_version_is_a_file_with_our_url_and_the_version_metadata() {
    // Arrange
    let wheel = "acme_core-1.0.0-py3-none-any.whl";
    let sdist = "acme_core-1.0.0.tar.gz";
    let first = stored(
        "Acme_Core",
        "1.0.0",
        100,
        vec![asset(wheel, 1, 10), asset(sdist, 2, 20)],
        r#"{"requires_python":">=3.8"}"#,
    );
    let mut newer = stored(
        "Acme_Core",
        "1.1",
        200,
        vec![asset("acme_core-1.1.tar.gz", 3, 30)],
        "{}",
    );
    newer.yanked = true;

    // Act
    let project = hosted(&pkg("Acme_Core"), &[first, newer], BASE);

    // Assert
    let python = Some(">=3.8".to_owned());
    let files = vec![
        DistFile {
            requires_python: python.clone(),
            ..expected(wheel, "1", 1, 10, 100)
        },
        DistFile {
            requires_python: python,
            ..expected(sdist, "1", 2, 20, 100)
        },
        DistFile {
            yanked: Yanked::Yes(None),
            ..expected("acme_core-1.1.tar.gz", "1.1", 3, 30, 200)
        },
    ];
    let versions = vec!["1".to_owned(), "1.1".to_owned()];
    assert_eq!(
        project,
        Project {
            name: pkg("acme-core"),
            versions,
            files
        }
    );
}

#[test]
fn unreadable_version_metadata_hides_nothing_but_requires_python() {
    // Arrange
    let files = vec![asset("acme-1.0.tar.gz", 1, 1)];
    let versions = vec![stored("acme", "1.0", 1, files, "not json")];

    // Act
    let project = hosted(&pkg("acme"), &versions, BASE);

    // Assert
    let requires_python: Vec<_> = project
        .files
        .iter()
        .map(|f| f.requires_python.clone())
        .collect();
    assert_eq!(requires_python, vec![None]);
}
