use crate::prelude::*;

/// 明示的な `limit` の上限。HTTP Range は stream で返すため切り詰めない。
const MAX_READ_BYTES: u64 = 128 * 1024 * 1024;
const DEFAULT_READ_BYTES: u64 = 1024 * 1024;

/// 読み取り結果の byte 列と、`Range` に応えた場合の範囲。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileContent {
    pub bytes: Vec<u8>,
    pub total_len: u64,
    /// `Range` に応えた場合の先頭と末尾（末尾を含む）。
    pub range: Option<(u64, u64)>,
}

impl FileContent {
    pub fn content_range(&self) -> Option<String> {
        self.range.map(|(start, end)| format!("bytes {start}-{end}/{}", self.total_len))
    }
}

/// 読み取る範囲の指定。`Range` header があればそれを、なければ `offset` と `limit` を使う。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadRange {
    /// 範囲も読み取り窓も指定しなければ file 全体を返す。
    Full,
    /// `Range` header の単一範囲。応答は部分応答になる。
    Bytes(ByteRange),
    /// `offset` と `limit` の範囲。file の末尾で切り詰め、応答は通常の応答になる。
    Window { offset: u64, limit: u64 },
}

impl ReadRange {
    /// `Range` header が解釈できない場合は、RFC 7233 に従い header を無視して `offset` と `limit` を使う。
    pub fn parse(range: Option<&str>, offset: Option<u64>, limit: Option<u64>) -> Result<Self> {
        if let Some(range) = range.and_then(ByteRange::parse) {
            return Ok(Self::Bytes(range));
        }

        if offset.is_none() && limit.is_none() {
            return Ok(Self::Full);
        }
        let limit = limit.unwrap_or(DEFAULT_READ_BYTES);
        if limit == 0 || limit > MAX_READ_BYTES {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("limit must be 1-{MAX_READ_BYTES}: {limit}")));
        }
        Ok(Self::Window {
            offset: offset.unwrap_or(0),
            limit,
        })
    }

    /// 長さ `total_len` の file に当てはめ、先頭と末尾（末尾を含まない）を返す。
    pub fn resolve(&self, total_len: u64) -> Result<(u64, u64)> {
        match self {
            Self::Full => Ok((0, total_len)),
            Self::Bytes(range) => range
                .resolve(total_len)
                .map(|(start, end)| (start, end + 1))
                .ok_or_else(|| Error::new(ErrorKind::RangeNotSatisfiable).with_message(format!("range is outside of {total_len} bytes"))),
            Self::Window { offset, limit } => {
                let start = (*offset).min(total_len);
                Ok((start, start.saturating_add(*limit).min(total_len)))
            }
        }
    }

    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Bytes(_))
    }
}

/// RFC 7233 の単一 byte range。file の長さを当てはめる前の形で持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ByteRange {
    /// `bytes=first-last`。`first <= last` を満たす。
    FromTo { first: u64, last: u64 },
    /// `bytes=first-`。
    From { first: u64 },
    /// `bytes=-len`。末尾から `len` byte。
    Suffix { len: u64 },
}

impl ByteRange {
    /// header がない、書式が不正、複数範囲のいずれかなら `None` を返す。
    fn parse(value: &str) -> Option<Self> {
        let spec = value.trim().strip_prefix("bytes=")?;
        if spec.contains(',') {
            return None;
        }
        let (first, last) = spec.trim().split_once('-')?;

        match (first.trim(), last.trim()) {
            ("", "") => None,
            ("", len) => len.parse().ok().map(|len| Self::Suffix { len }),
            (first, "") => first.parse().ok().map(|first| Self::From { first }),
            (first, last) => match (first.parse(), last.parse()) {
                (Ok(first), Ok(last)) if first <= last => Some(Self::FromTo { first, last }),
                _ => None,
            },
        }
    }

    /// 先頭と末尾（末尾を含む）を返す。満たせない範囲なら `None` を返す。
    fn resolve(&self, total_len: u64) -> Option<(u64, u64)> {
        let last_index = total_len.saturating_sub(1);
        let (start, end) = match *self {
            Self::FromTo { first, last } => (first, last.min(last_index)),
            Self::From { first } => (first, last_index),
            Self::Suffix { len: 0 } => return None,
            Self::Suffix { len } => (total_len.saturating_sub(len), last_index),
        };

        if total_len == 0 || start >= total_len {
            return None;
        }
        Some((start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(value: &str, total_len: u64) -> Option<Option<(u64, u64)>> {
        ByteRange::parse(value).map(|range| range.resolve(total_len))
    }

    #[test]
    fn parses_byte_ranges_test() {
        assert_eq!(resolve("bytes=0-4", 10), Some(Some((0, 4))));
        assert_eq!(resolve("bytes=5-", 10), Some(Some((5, 9))));
        assert_eq!(resolve("bytes=-3", 10), Some(Some((7, 9))));
        assert_eq!(resolve("bytes=8-100", 10), Some(Some((8, 9))));
        assert_eq!(resolve("bytes=10-", 10), Some(None));
        assert_eq!(resolve("bytes=-0", 10), Some(None));
        assert_eq!(resolve("bytes=0-1,4-5", 10), None);
        assert_eq!(resolve("items=0-1", 10), None);
        assert_eq!(resolve("bytes=5-1", 10), None);
    }

    #[test]
    fn falls_back_to_window_without_valid_range_test() -> testresult::TestResult {
        let range = ReadRange::parse(Some("items=0-1"), Some(4), Some(100))?;
        assert_eq!(range, ReadRange::Window { offset: 4, limit: 100 });
        assert!(!range.is_partial());
        assert_eq!(range.resolve(10)?, (4, 10));

        let range = ReadRange::parse(Some("bytes=1-3"), None, Some(0))?;
        assert!(range.is_partial());
        assert_eq!(range.resolve(10)?, (1, 4));
        assert_eq!(*range.resolve(0).unwrap_err().kind(), ErrorKind::RangeNotSatisfiable);

        assert_eq!(*ReadRange::parse(None, None, Some(0)).unwrap_err().kind(), ErrorKind::InvalidInput);
        Ok(())
    }
}
