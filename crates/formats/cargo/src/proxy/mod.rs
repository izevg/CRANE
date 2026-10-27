//! Знание crates.io для proxy-репозитория: хосты, политика кэша индекса и поиск архива в индексе.
//!
//! Это не протокол sparse вообще (он в `index`), а свойства конкретного upstream.

use std::borrow::Cow;
use std::num::NonZeroU32;
use std::time::Duration;

use crane_domain::proxy::{MetadataPolicy, NegativePolicy, UpstreamPath};
use crane_domain::registry::Sha256;
use serde::Deserialize;

pub const INDEX_HOST: &str = "index.crates.io";
pub const DL_HOST: &str = "static.crates.io";

const MINUTE: u64 = 60;
const DAY: u64 = 24 * 60 * MINUTE;
/// N = 3 из 11.1. Через `MIN + 2`, потому что `unwrap` в рабочем коде запрещён, а `new(3)` возвращает `Option`.
const ADAPTIVE_AFTER: NonZeroU32 = NonZeroU32::MIN.saturating_add(2);

/// Строка 11.1 «Cargo sparse index»: TTL 5 мин, stale-while-revalidate 5 мин, stale-if-error 7 дней,
/// negative 1 мин с ростом до 1 ч; N = 3.
///
/// Первый сегмент путей архивов (`crates`, 6 символов) не совпадает ни с одним первым сегментом пути
/// индекса (`1`, `2`, `3` или две буквы), поэтому ключи индекса и архивов под одним `RepositoryId`
/// не пересекаются.
pub const INDEX: MetadataPolicy = MetadataPolicy {
    ttl: Duration::from_secs(5 * MINUTE),
    revalidate_window: Duration::from_secs(5 * MINUTE),
    stale_if_error: Duration::from_secs(7 * DAY),
    adaptive_after: ADAPTIVE_AFTER,
    negative: NegativePolicy {
        ttl: Duration::from_secs(MINUTE),
        max: Duration::from_secs(60 * MINUTE),
    },
};

/// Где лежит архив версии и какой контрольной суммы он обязан быть.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Archive {
    pub path: UpstreamPath,
    pub sha256: Sha256,
}

/// Читаемая часть строки индекса; остальные поля игнорируются (совместимость вперёд).
#[derive(Deserialize)]
struct Line<'a> {
    // `Cow`: при экранировании в JSON строку нельзя занять из входа без копии.
    #[serde(borrow)]
    name: Cow<'a, str>,
    #[serde(borrow)]
    vers: Cow<'a, str>,
    #[serde(borrow)]
    cksum: Cow<'a, str>,
}

/// Архив версии по телу индекса upstream.
///
/// Путь строится из имени в строке индекса (регистр upstream, `Inflector/Inflector-0.11.4.crate`),
/// а не из имени в URL запроса: всё, что уходит в сеть, берётся из подписанного контрольной суммой источника.
/// Непарсящиеся строки и строки с битой суммой пропускаются, как это делает сам cargo, поэтому результат —
/// `Option`. Версия сравнивается побайтно: cargo подставляет в URL именно её. При нескольких строках
/// одной версии берётся последняя.
#[must_use]
pub fn archive(index: &[u8], version: &str) -> Option<Archive> {
    index
        .split(|byte| *byte == b'\n')
        .rev()
        .filter_map(|raw| serde_json::from_slice::<Line<'_>>(raw).ok())
        .filter(|line| line.vers == version)
        .find_map(|line| {
            let sha256 = Sha256::from_hex(&line.cksum).ok()?;
            let path =
                UpstreamPath::parse(&format!("crates/{0}/{0}-{1}.crate", line.name, line.vers))
                    .ok()?;
            Some(Archive { path, sha256 })
        })
}
