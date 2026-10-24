use sha3::{Digest as _, Sha3_256};

/// キャッシュの形式や変換の実装を変えたときに上げる。古い版のキーは、どの要求からも引かれなくなり、やがて LRU で消える。
pub const THUMBNAIL_FORMAT_VERSION: u32 = 3;

/// 各 field を長さ付きで SHA3-256 に入れる。field の切れ目が曖昧にならないよう、境界を含めて直列化する。
pub struct ThumbnailKeyHasher(Sha3_256);

impl ThumbnailKeyHasher {
    pub fn new() -> Self {
        let mut hasher = Self(Sha3_256::new());
        hasher.field(&THUMBNAIL_FORMAT_VERSION.to_be_bytes());
        hasher
    }

    pub fn field(&mut self, value: &[u8]) -> &mut Self {
        self.0.update((value.len() as u64).to_be_bytes());
        self.0.update(value);
        self
    }

    pub fn finish(self) -> ThumbnailKey {
        ThumbnailKey::from_digest(&self.0.finalize().into())
    }
}

/// サムネイルのキャッシュを指す SHA3-256 値。ファイル名にそのまま使えるよう、16 進文字列で持つ。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThumbnailKey(String);

impl ThumbnailKey {
    const HEX_LENGTH: usize = 64;

    pub fn from_digest(digest: &[u8; 32]) -> Self {
        Self(digest.iter().map(|byte| format!("{byte:02x}")).collect())
    }

    /// 保存された文字列を読み戻す。キャッシュの file 名は信用せず、形が違えば `None` にする。
    pub fn from_hex(hex: &str) -> Option<Self> {
        let valid = hex.len() == Self::HEX_LENGTH && hex.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
        valid.then(|| Self(hex.to_string()))
    }

    pub fn hex(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_hex_test() {
        let key = ThumbnailKey::from_digest(&[0xab; 32]);
        assert_eq!(key.hex().len(), 64);
        assert_eq!(ThumbnailKey::from_hex(key.hex()), Some(key));
    }

    #[test]
    fn distinguishes_field_boundaries_test() {
        let hash = |fields: &[&[u8]]| {
            let mut hasher = ThumbnailKeyHasher::new();
            for field in fields {
                hasher.field(field);
            }
            hasher.finish()
        };
        assert_ne!(hash(&[b"ab", b"c"]), hash(&[b"a", b"bc"]));
        assert_eq!(hash(&[b"ab", b"c"]), hash(&[b"ab", b"c"]));
    }

    #[test]
    fn rejects_malformed_hex_test() {
        for hex in ["", "ab", &"g".repeat(64), &"A".repeat(64), &"a".repeat(65), "../../etc/passwd"] {
            assert_eq!(ThumbnailKey::from_hex(hex), None, "{hex}");
        }
    }
}
