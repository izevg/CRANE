use crane_domain::registry::{OrderKey, OrderKeyBuilder, Version, VersionError};

/// Ранг pre-release ниже ранга релиза того же ядра.
const RANK_PRERELEASE: u8 = 0;
const RANK_RELEASE: u8 = 1;
/// Числовые идентификаторы pre-release младше буквенных.
const RANK_NUMERIC: u8 = 0;
const RANK_ALPHANUMERIC: u8 = 1;

/// Строгий `SemVer` 2.0. Build metadata не входит ни в каноническую форму, ни в порядок.
pub(crate) fn parse_version(raw: &str) -> Result<Version, VersionError> {
    let mut parsed = semver::Version::parse(raw).map_err(|e| VersionError::Invalid {
        raw: raw.to_owned(),
        reason: e.to_string(),
    })?;
    // SemVer 2.0: build metadata не участвует в precedence.
    parsed.build = semver::BuildMetadata::EMPTY;
    Ok(Version::from_parts(
        parsed.to_string(),
        raw.to_owned(),
        order_key(&parsed),
        !parsed.pre.is_empty(),
    ))
}

/// major, minor, patch; затем ранг (pre-release < релиз) и идентификаторы pre-release:
/// числовые < буквенных, числовые по значению, при общем префиксе длиннее список больше.
fn order_key(version: &semver::Version) -> OrderKey {
    let core = OrderKeyBuilder::new()
        .number(version.major)
        .number(version.minor)
        .number(version.patch);
    if version.pre.is_empty() {
        return core.rank(RANK_RELEASE).finish();
    }
    version
        .pre
        .as_str()
        .split('.')
        .fold(core.rank(RANK_PRERELEASE), |builder, id| {
            if id.bytes().all(|b| b.is_ascii_digit()) {
                // Число произвольной длины: сначала длина цифр (без ведущих нулей по SemVer), затем цифры.
                builder.rank(RANK_NUMERIC).number(id.len() as u64).text(id)
            } else {
                builder.rank(RANK_ALPHANUMERIC).text(id)
            }
        })
        .finish()
}
