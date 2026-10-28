use crane_domain::registry::{OrderKey, OrderKeyBuilder, Version, VersionError};
use pep440_rs::{LocalSegment, Prerelease, PrereleaseKind};

// Ранги рассчитаны на сравнение побайтово: маркер компонента больше терминатора, поэтому `1 < 1.0.1`.
const END: u8 = 0;
const COMPONENT: u8 = 1;
/// Только dev без pre и post (`1.0.dev1`) младше любого pre-release того же релиза.
const ONLY_DEV: u8 = 0;
const ALPHA: u8 = 1;
const BETA: u8 = 2;
const RC: u8 = 3;
const NO_PRE: u8 = 4;
const NO_POST: u8 = 0;
const POST: u8 = 1;
const DEV: u8 = 0;
const NO_DEV: u8 = 1;
/// Буквенный сегмент local младше числового.
const LOCAL_TEXT: u8 = 1;
const LOCAL_NUMBER: u8 = 2;

/// Версия по PEP 440. Каноническая форма — как у Warehouse (`canonicalize_version`): release без
/// хвостовых нулей, остальное нормализовано. По ней уникальна версия в БД, поэтому `1.0` и `1.0.0`
/// (колесо и sdist одного релиза) — одна версия.
pub(crate) fn parse_version(raw: &str) -> Result<Version, VersionError> {
    let parsed: pep440_rs::Version =
        raw.parse()
            .map_err(|e: pep440_rs::VersionParseError| VersionError::Invalid {
                raw: raw.to_owned(),
                reason: e.to_string(),
            })?;
    let release = trimmed(parsed.release());
    let canonical = parsed
        .clone()
        .with_release(release.iter().copied())
        .to_string();
    Ok(Version::from_parts(
        canonical,
        raw.to_owned(),
        order_key(&parsed, release),
        // dev тоже pre-release по PEP 440.
        parsed.any_prerelease(),
    ))
}

/// Release без хвостовых нулей, но не короче одного компонента (`0` остаётся `0`).
fn trimmed(release: &[u64]) -> &[u64] {
    let end = release.iter().rposition(|n| *n != 0).map_or(1, |i| i + 1);
    &release[..end]
}

/// Порядок `packaging.version._cmpkey`: epoch, release, pre, post, dev, local.
fn order_key(version: &pep440_rs::Version, release: &[u64]) -> OrderKey {
    let key = release
        .iter()
        .fold(OrderKeyBuilder::new().number(version.epoch()), |key, n| {
            key.rank(COMPONENT).number(*n)
        })
        .rank(END);
    let key = pre_key(key, version);
    let key = match version.post() {
        None => key.rank(NO_POST),
        Some(n) => key.rank(POST).number(n),
    };
    let key = match version.dev() {
        Some(n) => key.rank(DEV).number(n),
        None => key.rank(NO_DEV),
    };
    version
        .local()
        .iter()
        .fold(key, |key, segment| match segment {
            LocalSegment::String(text) => key.rank(LOCAL_TEXT).text(text),
            LocalSegment::Number(n) => key.rank(LOCAL_NUMBER).number(*n),
        })
        .rank(END)
        .finish()
}

fn pre_key(key: OrderKeyBuilder, version: &pep440_rs::Version) -> OrderKeyBuilder {
    match version.pre() {
        Some(Prerelease { kind, number }) => {
            let rank = match kind {
                PrereleaseKind::Alpha => ALPHA,
                PrereleaseKind::Beta => BETA,
                PrereleaseKind::Rc => RC,
            };
            key.rank(rank).number(number)
        }
        None if version.post().is_none() && version.dev().is_some() => key.rank(ONLY_DEV),
        None => key.rank(NO_PRE),
    }
}
