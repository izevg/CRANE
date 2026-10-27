//! Контрольные суммы из `dist`: SRI-список `integrity` и hex-`shasum`.

use base64::Engine;
use base64::alphabet;
use base64::engine::DecodePaddingMode;
use base64::engine::general_purpose::{GeneralPurpose, GeneralPurposeConfig};
use crane_domain::registry::{Checksum, Sha1, Sha512};

/// SRI допускает base64 с хвостом `=` и без него.
const SRI_ENGINE: GeneralPurpose = GeneralPurpose::new(
    &alphabet::STANDARD,
    GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MalformedSri;

/// Разбирает SRI-список: `sha512-` и `sha1-` становятся суммами, прочие алгоритмы игнорируются.
/// Битая запись известного алгоритма — отказ: молча пропустить её значит ослабить проверку.
pub(crate) fn parse_sri(integrity: &str) -> Result<Vec<Checksum>, MalformedSri> {
    let mut sums = Vec::new();
    for entry in integrity.split_whitespace() {
        let (algorithm, rest) = entry.split_once('-').ok_or(MalformedSri)?;
        // Необязательные параметры SRI (`?opt`) в сумму не входят.
        let encoded = rest.split_once('?').map_or(rest, |(digest, _)| digest);
        let sum = match algorithm {
            "sha512" => Checksum::Sha512(Sha512::from(decode(encoded)?)),
            "sha1" => Checksum::Sha1(Sha1::from(decode(encoded)?)),
            _ => continue,
        };
        if !sums.contains(&sum) {
            sums.push(sum);
        }
    }
    Ok(sums)
}

fn decode<const N: usize>(encoded: &str) -> Result<[u8; N], MalformedSri> {
    let bytes = SRI_ENGINE.decode(encoded).map_err(|_| MalformedSri)?;
    <[u8; N]>::try_from(bytes).map_err(|_| MalformedSri)
}

/// Первая сумма sha512 списка.
pub(crate) fn sha512_of(sums: &[Checksum]) -> Option<Checksum> {
    sums.iter()
        .find(|sum| matches!(sum, Checksum::Sha512(_)))
        .copied()
}
