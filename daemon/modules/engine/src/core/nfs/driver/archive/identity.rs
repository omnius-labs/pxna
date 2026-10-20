use std::{fmt, path::Path, time::UNIX_EPOCH};

use sha3::{Digest as _, Sha3_256};

use crate::prelude::*;

/// cache identity の直列化形式の版。
///
/// filename decode、path normalization、extract semantics、検証条件を変えたら上げる。
/// 値が変わるとすべての identity と cache key が変わり、古い entry は miss になる。
pub const CACHE_FORMAT_VERSION: u32 = 1;

/// 最外の local file が処理中に変わっていないかを確かめるための値。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceFingerprint {
    pub size: u64,
    pub mtime_s: i64,
    pub mtime_ns: u32,
}

impl SourceFingerprint {
    pub fn from_metadata(metadata: &std::fs::Metadata) -> Result<Self> {
        let modified = metadata.modified()?;
        let (mtime_s, mtime_ns) = match modified.duration_since(UNIX_EPOCH) {
            Ok(duration) => (duration.as_secs() as i64, duration.subsec_nanos()),
            Err(error) => {
                let before = error.duration();
                if before.subsec_nanos() == 0 {
                    (-(before.as_secs() as i64), 0)
                } else {
                    (-(before.as_secs() as i64) - 1, 1_000_000_000 - before.subsec_nanos())
                }
            }
        };
        Ok(Self {
            size: metadata.len(),
            mtime_s,
            mtime_ns,
        })
    }

    pub async fn read(path: &Path) -> Result<Self> {
        let metadata = tokio::fs::metadata(path).await?;
        Self::from_metadata(&metadata)
    }

    /// `path` の現在の fingerprint がこの値と一致しなければ `SourceChanged` を返す。
    pub async fn ensure_unchanged(&self, path: &Path) -> Result<()> {
        let current = match tokio::fs::metadata(path).await {
            Ok(metadata) => Some(Self::from_metadata(&metadata)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        if current.as_ref() == Some(self) {
            return Ok(());
        }
        Err(Error::new(ErrorKind::SourceChanged).with_message(format!("source archive changed: {}", path.display())))
    }
}

/// 1 つの archive を指す SHA3-256 値。
///
/// 最外の archive は local file の絶対 path と fingerprint から、内側の archive は親の identity と
/// 内側 path から導く。中間 archive の実体 file の path や更新時刻には依存しない。
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArchiveIdentity([u8; 32]);

impl ArchiveIdentity {
    pub fn from_local(path: &str, fingerprint: &SourceFingerprint) -> Self {
        let mut hasher = IdentityHasher::new("local");
        hasher.field(path.as_bytes());
        hasher.field(&fingerprint.size.to_le_bytes());
        hasher.field(&fingerprint.mtime_s.to_le_bytes());
        hasher.field(&fingerprint.mtime_ns.to_le_bytes());
        Self(hasher.finish())
    }

    /// 親 archive から取り出した entry を archive として開くときの identity。
    ///
    /// その entry の `EntryCacheKey` と同じ値になる。
    pub fn from_parent(parent: &ArchiveIdentity, inner_path: &str) -> Self {
        Self(EntryCacheKey::new(parent, inner_path).0)
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for ArchiveIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ArchiveIdentity({})", self.to_hex())
    }
}

/// archive 内の 1 entry を extraction cache 上で指す key。
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryCacheKey([u8; 32]);

impl EntryCacheKey {
    pub fn new(archive: &ArchiveIdentity, inner_path: &str) -> Self {
        let mut hasher = IdentityHasher::new("nested");
        hasher.field(&archive.0);
        hasher.field(inner_path.as_bytes());
        Self(hasher.finish())
    }

    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    pub fn from_hex(value: &str) -> Option<Self> {
        let bytes = hex::decode(value).ok()?;
        Some(Self(bytes.try_into().ok()?))
    }
}

impl fmt::Debug for EntryCacheKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EntryCacheKey({})", self.to_hex())
    }
}

/// field ごとに長さを前置して連結し、境界の曖昧さなく hash する。
struct IdentityHasher {
    inner: Sha3_256,
}

impl IdentityHasher {
    fn new(domain: &str) -> Self {
        Self::with_version(CACHE_FORMAT_VERSION, domain)
    }

    fn with_version(version: u32, domain: &str) -> Self {
        let mut hasher = Self { inner: Sha3_256::new() };
        hasher.field(&version.to_le_bytes());
        hasher.field(domain.as_bytes());
        hasher
    }

    fn field(&mut self, bytes: &[u8]) {
        self.inner.update((bytes.len() as u64).to_le_bytes());
        self.inner.update(bytes);
    }

    fn finish(self) -> [u8; 32] {
        self.inner.finalize().into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fingerprint() -> SourceFingerprint {
        SourceFingerprint {
            size: 10,
            mtime_s: 1_700_000_000,
            mtime_ns: 5,
        }
    }

    #[test]
    fn from_parent_equals_entry_cache_key_test() {
        let parent = ArchiveIdentity::from_local("/library/a.zip", &fingerprint());
        assert_eq!(ArchiveIdentity::from_parent(&parent, "x").0, EntryCacheKey::new(&parent, "x").0);
    }

    #[test]
    fn identity_changes_with_source_fields_test() {
        let base = ArchiveIdentity::from_local("/library/a.zip", &fingerprint());
        assert_ne!(base, ArchiveIdentity::from_local("/library/b.zip", &fingerprint()));
        assert_ne!(base, ArchiveIdentity::from_local("/library/a.zip", &SourceFingerprint { size: 11, ..fingerprint() }));
        assert_ne!(
            base,
            ArchiveIdentity::from_local(
                "/library/a.zip",
                &SourceFingerprint {
                    mtime_s: 1_700_000_001,
                    ..fingerprint()
                }
            )
        );
        assert_ne!(base, ArchiveIdentity::from_local("/library/a.zip", &SourceFingerprint { mtime_ns: 6, ..fingerprint() }));
    }

    #[test]
    fn field_boundaries_are_unambiguous_test() {
        let parent = ArchiveIdentity::from_local("/a", &fingerprint());
        assert_ne!(EntryCacheKey::new(&parent, "ab/c"), EntryCacheKey::new(&parent, "a/bc"));
    }

    #[test]
    fn cache_format_version_changes_every_value_test() {
        let mut current = IdentityHasher::with_version(CACHE_FORMAT_VERSION, "local");
        current.field(b"/library/a.zip");
        let mut next = IdentityHasher::with_version(CACHE_FORMAT_VERSION + 1, "local");
        next.field(b"/library/a.zip");
        assert_ne!(current.finish(), next.finish());

        let mut current = IdentityHasher::with_version(CACHE_FORMAT_VERSION, "nested");
        current.field(b"x");
        let mut next = IdentityHasher::with_version(CACHE_FORMAT_VERSION + 1, "nested");
        next.field(b"x");
        assert_ne!(current.finish(), next.finish());
    }

    #[test]
    fn hex_round_trip_test() {
        let key = EntryCacheKey::new(&ArchiveIdentity::from_local("/a", &fingerprint()), "x");
        assert_eq!(EntryCacheKey::from_hex(&key.to_hex()), Some(key));
        assert_eq!(EntryCacheKey::from_hex("zz"), None);
    }
}
