//! Бинарный формат записи в Valkey (big-endian, версия 1):
//!
//! ```text
//! version u8 | fetched_at: secs u64, nanos u32 | tag u8 | состояние
//! Document: unchanged u32 | etag: флаг u8 [len u32, utf8] | max_age: флаг u8 [секунды u64] | body до конца
//! Blob:     sha256 [32] | size u64
//! Missing:  misses u32
//! ```
//!
//! Формат ручной: serde в адаптерах запрещён, а полей мало.

use std::time::{Duration, SystemTime};

use bytes::Bytes;
use crane_domain::registry::Sha256;
use crane_ports::proxy::{Cached, CachedState, ETag};
use crane_ports::registry::Digests;

const VERSION: u8 = 1;
const TAG_DOCUMENT: u8 = 0;
const TAG_BLOB: u8 = 1;
const TAG_MISSING: u8 = 2;
const ABSENT: u8 = 0;
const PRESENT: u8 = 1;
const NANOS_PER_SECOND: u32 = 1_000_000_000;
const DIGEST_LEN: usize = 32;

/// Ошибка кодека. Без данных записи: в них бывает содержимое upstream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(super) enum CodecError {
    #[error("неизвестная версия формата записи")]
    Version,
    #[error("неизвестный тег состояния записи")]
    Tag,
    #[error("недопустимый флаг присутствия поля")]
    Flag,
    #[error("запись усечена")]
    Truncated,
    #[error("etag не является UTF-8")]
    ETag,
    #[error("после записи остались лишние байты")]
    Trailing,
    #[error("время получения вне диапазона, представимого в формате")]
    Time,
    #[error("etag длиннее, чем допускает формат")]
    TooLong,
}

/// Верхняя оценка размера записи: тело до 64 мегабайт не должно расти удвоениями буфера.
fn encoded_len(state: &CachedState) -> usize {
    const HEADER: usize = 1 + 8 + 4 + 1;
    const DOCUMENT_FIXED: usize = 4 + 1 + 4 + 1 + 8;
    const BLOB: usize = DIGEST_LEN + 8;
    HEADER
        + match state {
            CachedState::Document { body, etag, .. } => {
                DOCUMENT_FIXED + body.len() + etag.as_ref().map_or(0, |etag| etag.as_str().len())
            }
            CachedState::Blob(_) => BLOB,
            CachedState::Missing { .. } => 4,
        }
}

pub(super) fn encode(entry: &Cached) -> Result<Vec<u8>, CodecError> {
    let since_epoch = entry
        .fetched_at
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_err(|_| CodecError::Time)?;
    let mut out = Vec::with_capacity(encoded_len(&entry.state));
    out.push(VERSION);
    out.extend(since_epoch.as_secs().to_be_bytes());
    out.extend(since_epoch.subsec_nanos().to_be_bytes());
    match &entry.state {
        CachedState::Document {
            body,
            etag,
            unchanged,
            max_age,
        } => {
            out.push(TAG_DOCUMENT);
            out.extend(unchanged.to_be_bytes());
            match etag {
                Some(etag) => {
                    let raw = etag.as_str().as_bytes();
                    let len = u32::try_from(raw.len()).map_err(|_| CodecError::TooLong)?;
                    out.push(PRESENT);
                    out.extend(len.to_be_bytes());
                    out.extend(raw);
                }
                None => out.push(ABSENT),
            }
            match max_age {
                Some(max_age) => {
                    out.push(PRESENT);
                    out.extend(max_age.as_secs().to_be_bytes());
                }
                None => out.push(ABSENT),
            }
            out.extend(body);
        }
        CachedState::Blob(digests) => {
            out.push(TAG_BLOB);
            out.extend(digests.sha256.as_bytes());
            out.extend(digests.size.to_be_bytes());
        }
        CachedState::Missing { misses } => {
            out.push(TAG_MISSING);
            out.extend(misses.to_be_bytes());
        }
    }
    Ok(out)
}

pub(super) fn decode(raw: Bytes) -> Result<Cached, CodecError> {
    let mut reader = Reader(raw);
    if reader.u8()? != VERSION {
        return Err(CodecError::Version);
    }
    let secs = reader.u64()?;
    let nanos = reader.u32()?;
    if nanos >= NANOS_PER_SECOND {
        return Err(CodecError::Time);
    }
    let fetched_at = SystemTime::UNIX_EPOCH
        .checked_add(Duration::new(secs, nanos))
        .ok_or(CodecError::Time)?;
    let state = match reader.u8()? {
        TAG_DOCUMENT => {
            let unchanged = reader.u32()?;
            let etag = if reader.flag()? {
                let len = usize::try_from(reader.u32()?).map_err(|_| CodecError::Truncated)?;
                let text =
                    String::from_utf8(reader.take(len)?.to_vec()).map_err(|_| CodecError::ETag)?;
                Some(ETag::new(text))
            } else {
                None
            };
            let max_age = if reader.flag()? {
                Some(Duration::from_secs(reader.u64()?))
            } else {
                None
            };
            CachedState::Document {
                body: reader.rest(),
                etag,
                unchanged,
                max_age,
            }
        }
        TAG_BLOB => {
            let sha256 = Sha256::from(reader.array::<DIGEST_LEN>()?);
            let size = reader.u64()?;
            reader.finish()?;
            CachedState::Blob(Digests { sha256, size })
        }
        TAG_MISSING => {
            let misses = reader.u32()?;
            reader.finish()?;
            CachedState::Missing { misses }
        }
        _ => return Err(CodecError::Tag),
    };
    Ok(Cached { fetched_at, state })
}

/// Последовательное чтение из буфера; `Bytes` позволяет отдать тело без копирования.
struct Reader(Bytes);

impl Reader {
    fn take(&mut self, len: usize) -> Result<Bytes, CodecError> {
        if self.0.len() < len {
            return Err(CodecError::Truncated);
        }
        Ok(self.0.split_to(len))
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], CodecError> {
        <[u8; N]>::try_from(&*self.take(N)?).map_err(|_| CodecError::Truncated)
    }

    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(u8::from_be_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    fn flag(&mut self) -> Result<bool, CodecError> {
        match self.u8()? {
            ABSENT => Ok(false),
            PRESENT => Ok(true),
            _ => Err(CodecError::Flag),
        }
    }

    fn rest(&mut self) -> Bytes {
        std::mem::take(&mut self.0)
    }

    fn finish(&self) -> Result<(), CodecError> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err(CodecError::Trailing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HEADER: [u8; 13] = [1, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0];

    fn framed(tail: &[u8]) -> Bytes {
        Bytes::from([HEADER.as_slice(), tail].concat())
    }

    #[test]
    fn decode_rejects_malformed_input() {
        // Arrange
        let cases = [
            ("неизвестная версия", Bytes::from_static(&[2])),
            ("пустой ввод", Bytes::new()),
            ("усечено fetched_at", Bytes::from_static(&[1, 0, 0, 0])),
            (
                "наносекунды вне диапазона",
                Bytes::from_static(&[1, 0, 0, 0, 0, 0, 0, 0, 7, 0x3b, 0x9a, 0xca, 0x00, 0]),
            ),
            ("неизвестный тег", framed(&[9])),
            ("усечён счётчик документа", framed(&[0, 0, 0])),
            ("неизвестный флаг etag", framed(&[0, 0, 0, 0, 0, 2])),
            ("усечена длина etag", framed(&[0, 0, 0, 0, 0, 1, 0, 0])),
            (
                "усечены байты etag",
                framed(&[0, 0, 0, 0, 0, 1, 0, 0, 0, 4, b'a', b'b']),
            ),
            (
                "etag не UTF-8",
                framed(&[0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0xff, 0]),
            ),
            ("неизвестный флаг max_age", framed(&[0, 0, 0, 0, 0, 0, 5])),
            ("max_age без 8 байт", framed(&[0, 0, 0, 0, 0, 0, 1, 0, 0])),
            ("хвост после Missing", framed(&[2, 0, 0, 0, 1, 0])),
            ("усечён Missing", framed(&[2, 0, 0])),
            ("усечён Blob", framed(&[1, 0, 0])),
            (
                "хвост после Blob",
                framed(&[&[1][..], &[0; 32], &[0; 8], &[0]].concat()),
            ),
        ];

        // Act
        let actual = cases.map(|(name, raw)| (name, decode(raw).err()));

        // Assert
        assert_eq!(
            actual,
            [
                ("неизвестная версия", Some(CodecError::Version)),
                ("пустой ввод", Some(CodecError::Truncated)),
                ("усечено fetched_at", Some(CodecError::Truncated)),
                ("наносекунды вне диапазона", Some(CodecError::Time)),
                ("неизвестный тег", Some(CodecError::Tag)),
                ("усечён счётчик документа", Some(CodecError::Truncated)),
                ("неизвестный флаг etag", Some(CodecError::Flag)),
                ("усечена длина etag", Some(CodecError::Truncated)),
                ("усечены байты etag", Some(CodecError::Truncated)),
                ("etag не UTF-8", Some(CodecError::ETag)),
                ("неизвестный флаг max_age", Some(CodecError::Flag)),
                ("max_age без 8 байт", Some(CodecError::Truncated)),
                ("хвост после Missing", Some(CodecError::Trailing)),
                ("усечён Missing", Some(CodecError::Truncated)),
                ("усечён Blob", Some(CodecError::Truncated)),
                ("хвост после Blob", Some(CodecError::Trailing)),
            ]
        );
    }

    #[test]
    fn encode_rejects_time_before_unix_epoch() {
        // Arrange
        let entry = Cached {
            fetched_at: SystemTime::UNIX_EPOCH - Duration::from_secs(1),
            state: CachedState::Missing { misses: 1 },
        };

        // Act
        let result = encode(&entry);

        // Assert
        assert_eq!(result, Err(CodecError::Time));
    }
}
