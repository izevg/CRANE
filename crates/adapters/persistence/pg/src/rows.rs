//! Отображение строк БД в доменные типы и обратно. SQL здесь нет.
//!
//! Типовое правило крейта: в запросы привязываются только `&str`, `String`, `i64`, `bool`,
//! `Uuid`, `OffsetDateTime` и `Vec<u8>`; все приведения (`::ver_state`, `::text`, `::json`)
//! делает SQL, поэтому собственных реализаций `Type`/`Encode`/`Decode` нет.

use std::fmt::Display;
use std::time::{Duration, SystemTime};

use crane_domain::registry::{
    Asset, Format, FormatMetadata, PackageName, PackageVersion, Repository, RepositoryId,
    RepositoryKind, Sha256, Version, VersionId, VersionState,
};
use crane_ports::registry::CatalogError;
use sqlx::postgres::PgRow;
use sqlx::{Decode, Row, Type};
use time::OffsetDateTime;
use uuid::Uuid;

/// Сбой драйвера или БД.
pub(crate) fn backend(error: sqlx::Error) -> CatalogError {
    CatalogError::Backend(Box::new(error))
}

/// Строка в БД не разбирается доменом: данные повреждены или схема разошлась с кодом.
pub(crate) fn corrupt(what: impl Display) -> CatalogError {
    CatalogError::Backend(format!("повреждённая запись каталога: {what}").into())
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
    let shifted = match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(after) => time::Duration::try_from(after)
            .ok()
            .and_then(|d| OffsetDateTime::UNIX_EPOCH.checked_add(d)),
        Err(before) => time::Duration::try_from(before.duration())
            .ok()
            .and_then(|d| OffsetDateTime::UNIX_EPOCH.checked_sub(d)),
    };
    shifted.ok_or_else(|| corrupt("момент времени вне диапазона timestamptz"))
}

fn from_db_time(time: OffsetDateTime) -> Result<SystemTime, CatalogError> {
    let delta = time - OffsetDateTime::UNIX_EPOCH;
    let magnitude: Duration = delta.unsigned_abs();
    let shifted = if delta.is_negative() {
        SystemTime::UNIX_EPOCH.checked_sub(magnitude)
    } else {
        SystemTime::UNIX_EPOCH.checked_add(magnitude)
    };
    shifted.ok_or_else(|| corrupt("момент времени вне диапазона SystemTime"))
}

fn column<'r, T>(row: &'r PgRow, name: &str) -> Result<T, CatalogError>
where
    T: Decode<'r, sqlx::Postgres> + Type<sqlx::Postgres>,
{
    row.try_get(name).map_err(backend)
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

/// Колонки: `format`, `name_display`, `id`, `repository_id`, `version_display`, `state`,
/// `yanked`, `published_at`, `metadata`, `paths`, `digests`, `sizes`; массивы файлов
/// выровнены по индексу и отсортированы по пути.
pub(crate) fn version_from_row(row: &PgRow) -> Result<PackageVersion, CatalogError> {
    let format = column::<&str>(row, "format")?
        .parse::<Format>()
        .map_err(corrupt)?;
    Ok(PackageVersion {
        id: VersionId::new(column::<Uuid>(row, "id")?),
        repository: RepositoryId::new(column::<Uuid>(row, "repository_id")?),
        package: PackageName::parse(format, column::<&str>(row, "name_display")?)
            .map_err(corrupt)?,
        version: Version::parse(format, column::<&str>(row, "version_display")?)
            .map_err(corrupt)?,
        state: state_parse(column::<&str>(row, "state")?)?,
        yanked: column::<bool>(row, "yanked")?,
        published_at: from_db_time(column::<OffsetDateTime>(row, "published_at")?)?,
        assets: assets_from_columns(
            column::<Vec<String>>(row, "paths")?,
            column::<Vec<Vec<u8>>>(row, "digests")?,
            column::<Vec<i64>>(row, "sizes")?,
        )?,
        metadata: FormatMetadata::new(column::<String>(row, "metadata")?),
    })
}

fn assets_from_columns(
    paths: Vec<String>,
    digests: Vec<Vec<u8>>,
    sizes: Vec<i64>,
) -> Result<Vec<Asset>, CatalogError> {
    if paths.len() != digests.len() || paths.len() != sizes.len() {
        return Err(corrupt("массивы файлов версии разной длины"));
    }
    paths
        .into_iter()
        .zip(digests)
        .zip(sizes)
        .map(|((path, digest), size)| {
            let bytes = <[u8; 32]>::try_from(digest)
                .map_err(|d| corrupt(format!("sha256 длиной {} вместо 32", d.len())))?;
            Ok(Asset {
                path,
                sha256: Sha256::from(bytes),
                size: u64::try_from(size).map_err(|_| corrupt("отрицательный размер файла"))?,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [RepositoryKind; 2] = [RepositoryKind::Hosted, RepositoryKind::Proxy];
    const STATES: [VersionState; 4] = [
        VersionState::Quarantined,
        VersionState::Released,
        VersionState::Blocked,
        VersionState::Deleted,
    ];

    #[test]
    fn repository_kind_round_trips_through_text() {
        // Arrange
        let kinds = KINDS;

        // Act
        let back = kinds.map(|k| kind_parse(kind_as_str(k)).ok());

        // Assert
        assert_eq!(back, kinds.map(Some));
    }

    #[test]
    fn repository_kind_texts_match_sql_enum() {
        // Arrange
        let kinds = KINDS;

        // Act
        let texts = kinds.map(kind_as_str);

        // Assert
        assert_eq!(texts, ["hosted", "proxy"]);
    }

    #[test]
    fn unknown_repository_kind_is_rejected() {
        // Arrange
        let text = "mirror";

        // Act
        let result = kind_parse(text);

        // Assert
        assert!(matches!(result, Err(CatalogError::Backend(_))));
    }

    #[test]
    fn version_state_round_trips_through_text() {
        // Arrange
        let states = STATES;

        // Act
        let back = states.map(|s| state_parse(state_as_str(s)).ok());

        // Assert
        assert_eq!(back, states.map(Some));
    }

    #[test]
    fn version_state_texts_match_sql_enum() {
        // Arrange
        let states = STATES;

        // Act
        let texts = states.map(state_as_str);

        // Assert
        assert_eq!(texts, ["quarantined", "released", "blocked", "deleted"]);
    }

    #[test]
    fn unknown_version_state_is_rejected() {
        // Arrange
        let text = "Released";

        // Act
        let result = state_parse(text);

        // Assert
        assert!(matches!(result, Err(CatalogError::Backend(_))));
    }

    #[test]
    fn system_time_round_trips_before_and_after_epoch() {
        // Arrange
        let moments = [
            SystemTime::UNIX_EPOCH - Duration::from_hours(24),
            SystemTime::UNIX_EPOCH,
            SystemTime::UNIX_EPOCH + Duration::from_micros(1_700_000_000_123_456),
        ];

        // Act
        let back = moments.map(|m| to_db_time(m).and_then(from_db_time).ok());

        // Assert
        assert_eq!(back, moments.map(Some));
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

    #[test]
    fn assets_with_mismatched_array_lengths_are_corrupt() {
        // Arrange
        let (paths, digests, sizes) = (vec!["a".to_owned()], vec![vec![0; 32]], vec![]);

        // Act
        let result = assets_from_columns(paths, digests, sizes);

        // Assert
        assert!(matches!(result, Err(CatalogError::Backend(_))));
    }

    #[test]
    fn asset_with_short_digest_is_corrupt() {
        // Arrange
        let (paths, digests, sizes) = (vec!["a".to_owned()], vec![vec![0; 31]], vec![1]);

        // Act
        let result = assets_from_columns(paths, digests, sizes);

        // Assert
        assert!(matches!(result, Err(CatalogError::Backend(_))));
    }

    #[test]
    fn asset_with_negative_size_is_corrupt() {
        // Arrange
        let (paths, digests, sizes) = (vec!["a".to_owned()], vec![vec![0; 32]], vec![-1]);

        // Act
        let result = assets_from_columns(paths, digests, sizes);

        // Assert
        assert!(matches!(result, Err(CatalogError::Backend(_))));
    }

    #[test]
    fn valid_columns_become_assets_in_input_order() {
        // Arrange
        let paths = vec!["a".to_owned(), "b".to_owned()];
        let digests = vec![vec![1; 32], vec![2; 32]];

        // Act
        let assets = assets_from_columns(paths, digests, vec![5, 0]).ok();

        // Assert
        let expected = vec![
            Asset {
                path: "a".to_owned(),
                sha256: Sha256::from([1; 32]),
                size: 5,
            },
            Asset {
                path: "b".to_owned(),
                sha256: Sha256::from([2; 32]),
                size: 0,
            },
        ];
        assert_eq!(assets, Some(expected));
    }
}
