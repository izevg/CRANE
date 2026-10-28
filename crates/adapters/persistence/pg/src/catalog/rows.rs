//! Отображение строк БД в доменные типы и обратно. SQL здесь нет.
//!
//! Здесь же единственная воронка ошибок драйвера (`backend`): она решает, что хранилище
//! не отвечает (`Unreachable`, клиенту 503), а что сбой или баг (`Backend`, клиенту 500).
//!
//! Типовое правило крейта: в запросы привязываются только `&str`, `String`, `i64`, `bool`,
//! `Uuid`, `OffsetDateTime` и `Vec<u8>`; все приведения (`::ver_state`, `::text`, `::json`)
//! делает SQL, поэтому собственных реализаций `Type`/`Encode`/`Decode` нет.

use std::fmt::Display;
use std::time::SystemTime;

use crane_domain::registry::{
    Asset, Digests, Format, FormatMetadata, Md5, Namespace, NamespacePattern, OrderKey,
    PackageName, PackageVersion, Repository, RepositoryId, RepositoryKind, Sha1, Sha256, Sha512,
    Version, VersionId, VersionState, WorkspaceId,
};
use crane_ports::registry::CatalogError;

use crate::db::{self, is_unreachable};
use sqlx::postgres::PgRow;
use sqlx::{Decode, Row, Type};
use time::OffsetDateTime;
use uuid::Uuid;

/// Входное значение нельзя сохранить в БД (не помещается в тип колонки): ошибка вызывающего,
/// но порт не различает её среди сбоев адаптера.
pub(crate) fn rejected(what: impl Display) -> CatalogError {
    CatalogError::Backend(format!("значение не сохраняется в каталоге: {what}").into())
}

/// Строка в БД не разбирается доменом: данные повреждены или схема разошлась с кодом.
pub(crate) fn corrupt(what: impl Display) -> CatalogError {
    CatalogError::Backend(format!("повреждённая запись каталога: {what}").into())
}

/// Сбой драйвера или БД: единственная воронка, где решается, «хранилище не отвечает» это или баг.
pub(crate) fn backend(error: sqlx::Error) -> CatalogError {
    if is_unreachable(&error) {
        CatalogError::Unreachable(Box::new(error))
    } else {
        CatalogError::Backend(Box::new(error))
    }
}

pub(crate) const fn kind_as_str(kind: RepositoryKind) -> &'static str {
    match kind {
        RepositoryKind::Hosted => "hosted",
        RepositoryKind::Proxy => "proxy",
    }
}

pub(crate) fn kind_parse(text: &str) -> Result<RepositoryKind, CatalogError> {
    match text {
        "hosted" => Ok(RepositoryKind::Hosted),
        "proxy" => Ok(RepositoryKind::Proxy),
        other => Err(corrupt(format!("неизвестный вид репозитория {other:?}"))),
    }
}

pub(crate) const fn state_as_str(state: VersionState) -> &'static str {
    match state {
        VersionState::Quarantined => "quarantined",
        VersionState::Released => "released",
        VersionState::Blocked => "blocked",
        VersionState::Deleted => "deleted",
    }
}

pub(crate) fn state_parse(text: &str) -> Result<VersionState, CatalogError> {
    match text {
        "quarantined" => Ok(VersionState::Quarantined),
        "released" => Ok(VersionState::Released),
        "blocked" => Ok(VersionState::Blocked),
        "deleted" => Ok(VersionState::Deleted),
        other => Err(corrupt(format!("неизвестное состояние версии {other:?}"))),
    }
}

/// `SystemTime` -> `timestamptz` без паник на крайних значениях.
pub(crate) fn to_db_time(time: SystemTime) -> Result<OffsetDateTime, CatalogError> {
    db::to_db_time(time).ok_or_else(|| rejected("момент времени вне диапазона timestamptz"))
}

fn from_db_time(time: OffsetDateTime) -> Result<SystemTime, CatalogError> {
    db::from_db_time(time).ok_or_else(|| corrupt("момент времени вне диапазона SystemTime"))
}

fn column<'r, T>(row: &'r PgRow, name: &str) -> Result<T, CatalogError>
where
    T: Decode<'r, sqlx::Postgres> + Type<sqlx::Postgres>,
{
    row.try_get(name).map_err(corrupt)
}

/// Колонки: `id`, `format`, `kind`, `name`.
pub(crate) fn repository_from_row(row: &PgRow) -> Result<Repository, CatalogError> {
    Ok(Repository {
        id: RepositoryId::new(column::<Uuid>(row, "id")?),
        format: column::<&str>(row, "format")?
            .parse::<Format>()
            .map_err(corrupt)?,
        kind: kind_parse(column::<&str>(row, "kind")?)?,
        name: column::<String>(row, "name")?,
    })
}

/// Колонки: `workspace_id`, `format`, `pattern`.
pub(crate) fn namespace_from_row(row: &PgRow) -> Result<Namespace, CatalogError> {
    let format = column::<&str>(row, "format")?
        .parse::<Format>()
        .map_err(corrupt)?;
    Ok(Namespace {
        format,
        // Хранится нормализованным: грамматику экосистемы адаптер не знает.
        pattern: NamespacePattern::from_stored(column::<String>(row, "pattern")?),
        workspace: WorkspaceId::new(column::<Uuid>(row, "workspace_id")?),
    })
}

/// Колонки: `name_norm`, `name_display`, `id`, `repository_id`, `version_canonical`,
/// `version_display`, `version_order`, `prerelease`, `state`,
/// `yanked`, `deprecated`, `published_at`, `metadata`, `paths`, `sha256`, `sha512`, `sha1`, `md5`, `sizes`; массивы файлов
/// выровнены по индексу и отсортированы по пути.
pub(crate) fn version_from_row(row: &PgRow) -> Result<PackageVersion, CatalogError> {
    Ok(PackageVersion {
        id: VersionId::new(column::<Uuid>(row, "id")?),
        repository: RepositoryId::new(column::<Uuid>(row, "repository_id")?),
        package: PackageName::from_parts(
            column::<String>(row, "name_norm")?,
            column::<String>(row, "name_display")?,
        ),
        version: Version::from_parts(
            column::<String>(row, "version_canonical")?,
            column::<String>(row, "version_display")?,
            OrderKey::from_bytes(column::<Vec<u8>>(row, "version_order")?),
            column::<bool>(row, "prerelease")?,
        ),
        state: state_parse(column::<&str>(row, "state")?)?,
        yanked: column::<bool>(row, "yanked")?,
        deprecated: column::<Option<String>>(row, "deprecated")?,
        published_at: from_db_time(column::<OffsetDateTime>(row, "published_at")?)?,
        assets: AssetColumns {
            paths: column::<Vec<String>>(row, "paths")?,
            sha256: column::<Vec<Vec<u8>>>(row, "sha256")?,
            sha512: column::<Vec<Vec<u8>>>(row, "sha512")?,
            sha1: column::<Vec<Vec<u8>>>(row, "sha1")?,
            md5: column::<Vec<Vec<u8>>>(row, "md5")?,
            sizes: column::<Vec<i64>>(row, "sizes")?,
        }
        .into_assets()?,
        metadata: FormatMetadata::new(column::<String>(row, "metadata")?),
    })
}

/// Параллельные массивы файлов версии из `array_agg`, выровненные по индексу.
struct AssetColumns {
    paths: Vec<String>,
    sha256: Vec<Vec<u8>>,
    sha512: Vec<Vec<u8>>,
    sha1: Vec<Vec<u8>>,
    md5: Vec<Vec<u8>>,
    sizes: Vec<i64>,
}

/// Дайджест фиксированной длины из байт колонки; чужая длина - повреждение данных
/// (CHECK в схеме её не допускает).
fn fixed<const N: usize>(name: &str, bytes: Vec<u8>) -> Result<[u8; N], CatalogError> {
    <[u8; N]>::try_from(bytes).map_err(|b| corrupt(format!("{name} длиной {} вместо {N}", b.len())))
}

impl AssetColumns {
    fn into_assets(self) -> Result<Vec<Asset>, CatalogError> {
        let len = self.paths.len();
        let lens = [
            self.sha256.len(),
            self.sha512.len(),
            self.sha1.len(),
            self.md5.len(),
            self.sizes.len(),
        ];
        if lens.iter().any(|&l| l != len) {
            return Err(corrupt("массивы файлов версии разной длины"));
        }
        self.paths
            .into_iter()
            .zip(self.sha256)
            .zip(self.sha512)
            .zip(self.sha1)
            .zip(self.md5)
            .zip(self.sizes)
            .map(|(((((path, sha256), sha512), sha1), md5), size)| {
                Ok(Asset {
                    path,
                    content: Digests {
                        sha256: Sha256::from(fixed("sha256", sha256)?),
                        sha512: Sha512::from(fixed("sha512", sha512)?),
                        sha1: Sha1::from(fixed("sha1", sha1)?),
                        md5: Md5::from(fixed("md5", md5)?),
                        size: u64::try_from(size)
                            .map_err(|_| corrupt("отрицательный размер файла"))?,
                    },
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn repository_kind_codec_matches_sql_enum_both_ways() {
        // Arrange
        let kinds = [RepositoryKind::Hosted, RepositoryKind::Proxy];

        // Act
        let actual = kinds.map(|k| (kind_as_str(k), kind_parse(kind_as_str(k)).ok()));

        // Assert
        assert_eq!(
            actual,
            [
                ("hosted", Some(RepositoryKind::Hosted)),
                ("proxy", Some(RepositoryKind::Proxy)),
            ]
        );
    }

    #[test]
    fn version_state_codec_matches_sql_enum_both_ways() {
        // Arrange
        let states = [
            VersionState::Quarantined,
            VersionState::Released,
            VersionState::Blocked,
            VersionState::Deleted,
        ];

        // Act
        let actual = states.map(|s| (state_as_str(s), state_parse(state_as_str(s)).ok()));

        // Assert
        assert_eq!(
            actual,
            [
                ("quarantined", Some(VersionState::Quarantined)),
                ("released", Some(VersionState::Released)),
                ("blocked", Some(VersionState::Blocked)),
                ("deleted", Some(VersionState::Deleted)),
            ]
        );
    }

    #[test]
    fn unknown_enum_text_is_rejected_as_corrupt_row() {
        // Arrange
        let unknown = ["mirror", "Hosted", "", "released "];

        // Act
        let rejected = unknown.map(|t| (kind_parse(t).is_err(), state_parse(t).is_err()));

        // Assert
        assert_eq!(rejected, [(true, true); 4]);
    }

    #[test]
    fn driver_errors_split_into_unreachable_and_backend() {
        // Arrange
        let cases = [
            sqlx::Error::PoolTimedOut,
            sqlx::Error::PoolClosed,
            sqlx::Error::WorkerCrashed,
            sqlx::Error::Io(std::io::Error::other("connection refused")),
            sqlx::Error::RowNotFound,
            sqlx::Error::ColumnNotFound("x".to_owned()),
        ];

        // Act
        let actual = cases.map(|error| matches!(backend(error), CatalogError::Unreachable(_)));

        // Assert
        assert_eq!(actual, [true, true, true, true, false, false]);
    }

    #[test]
    fn time_beyond_timestamptz_range_is_rejected() {
        // Arrange
        let far = SystemTime::UNIX_EPOCH + Duration::from_secs(u64::MAX / 2);

        // Act
        let result = to_db_time(far);

        // Assert
        assert!(matches!(result, Err(CatalogError::Backend(_))));
    }

    fn columns(n: usize) -> AssetColumns {
        AssetColumns {
            paths: vec!["a".to_owned(); n],
            sha256: vec![vec![1; 32]; n],
            sha512: vec![vec![2; 64]; n],
            sha1: vec![vec![3; 20]; n],
            md5: vec![vec![4; 16]; n],
            sizes: vec![5; n],
        }
    }

    #[test]
    fn inconsistent_asset_columns_are_rejected_as_corrupt_row() {
        // Arrange
        let cases = [
            AssetColumns {
                sizes: vec![],
                ..columns(1)
            },
            AssetColumns {
                sha256: vec![vec![1; 31]],
                ..columns(1)
            },
            AssetColumns {
                sha512: vec![vec![2; 63]],
                ..columns(1)
            },
            AssetColumns {
                sha1: vec![vec![3; 19]],
                ..columns(1)
            },
            AssetColumns {
                md5: vec![vec![4; 15]],
                ..columns(1)
            },
            AssetColumns {
                md5: vec![],
                ..columns(1)
            },
            AssetColumns {
                sizes: vec![-1],
                ..columns(1)
            },
        ];

        // Act
        let rejected = cases.map(|c| c.into_assets().is_err());

        // Assert
        assert_eq!(rejected, [true; 7]);
    }

    #[test]
    fn valid_columns_become_assets_in_input_order() {
        // Arrange
        let input = AssetColumns {
            paths: vec!["a".to_owned(), "b".to_owned()],
            sizes: vec![5, 0],
            ..columns(2)
        };

        // Act
        let assets = input.into_assets().ok();

        // Assert
        let content = |size| Digests {
            sha256: Sha256::from([1; 32]),
            sha512: Sha512::from([2; 64]),
            sha1: Sha1::from([3; 20]),
            md5: Md5::from([4; 16]),
            size,
        };
        let expected = vec![
            Asset {
                path: "a".to_owned(),
                content: content(5),
            },
            Asset {
                path: "b".to_owned(),
                content: content(0),
            },
        ];
        assert_eq!(assets, Some(expected));
    }
}
