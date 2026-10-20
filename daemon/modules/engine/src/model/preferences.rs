use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// サムネイルの形。タイルに切り抜いて出すか、全体を収めて出すか。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThumbnailShape {
    Crop,
    Fit,
}

/// 一覧の見せ方。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ListView {
    Grid,
    List,
}

/// ビューアが表示用画像から元の画像へ差し替える時機。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OriginalImageMode {
    /// 自動で差し替える。
    Auto,
    /// 拡大したときだけ差し替える。
    OnZoom,
}

/// 並べ替えのキー。一覧の API の `sort` と同じ値を取る。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortKey {
    Name,
    Kind,
    Size,
    ModifiedAt,
}

/// 並べ替えの向き。一覧の API の `order` と同じ値を取る。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortOrder {
    Asc,
    Desc,
}

/// 画面の幅の区分ごとに持つ値の組。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplaySet {
    /// タイルの大きさの段階（1 から 5）。
    pub tile_size: u8,
    pub thumbnail_shape: ThumbnailShape,
    pub view: ListView,
    pub original_image: OriginalImageMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortPreference {
    pub key: SortKey,
    pub order: SortOrder,
}

/// オーナーが選んでサーバに保存する、web の見た目と振る舞いの値の組。
///
/// 広い画面と狭い画面の 2 組と、画面の幅に関係しない並べ替えを持つ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub wide: DisplaySet,
    pub narrow: DisplaySet,
    pub sort: SortPreference,
}

impl DisplaySet {
    pub const MIN_TILE_SIZE: u8 = 1;
    pub const MAX_TILE_SIZE: u8 = 5;

    fn validate(&self, name: &str) -> Result<()> {
        if !(Self::MIN_TILE_SIZE..=Self::MAX_TILE_SIZE).contains(&self.tile_size) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("{name}.tileSize must be between {} and {}", Self::MIN_TILE_SIZE, Self::MAX_TILE_SIZE)));
        }
        Ok(())
    }
}

impl Preferences {
    /// 値の範囲を確かめる。列挙の値は、読み込みの時点で既知の値に限られる。
    pub fn validate(&self) -> Result<()> {
        self.wide.validate("wide")?;
        self.narrow.validate("narrow")
    }
}

impl Default for Preferences {
    /// 保存したことがないときの値。狭い画面は、回線が細いことを見込んで元の画像への差し替えを拡大したときだけにする。
    fn default() -> Self {
        let wide = DisplaySet {
            tile_size: 3,
            thumbnail_shape: ThumbnailShape::Crop,
            view: ListView::Grid,
            original_image: OriginalImageMode::Auto,
        };
        Self {
            wide,
            narrow: DisplaySet {
                original_image: OriginalImageMode::OnZoom,
                ..wide
            },
            sort: SortPreference {
                key: SortKey::Name,
                order: SortOrder::Asc,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_the_design_test() {
        let preferences = Preferences::default();
        assert_eq!(preferences.wide.tile_size, 3);
        assert_eq!(preferences.wide.original_image, OriginalImageMode::Auto);
        assert_eq!(preferences.narrow.original_image, OriginalImageMode::OnZoom);
        assert_eq!((preferences.sort.key, preferences.sort.order), (SortKey::Name, SortOrder::Asc));
        assert!(preferences.validate().is_ok());
    }

    #[test]
    fn rejects_a_tile_size_out_of_range_test() {
        for tile_size in [0, 6, 255] {
            for narrow in [false, true] {
                let mut preferences = Preferences::default();
                let set = if narrow { &mut preferences.narrow } else { &mut preferences.wide };
                set.tile_size = tile_size;
                assert_eq!(*preferences.validate().unwrap_err().kind(), ErrorKind::InvalidInput, "{tile_size} {narrow}");
            }
        }
    }

    #[test]
    fn serializes_with_the_api_names_test() -> testresult::TestResult {
        let json = serde_json::to_value(Preferences::default())?;
        assert_eq!(json["narrow"]["originalImage"], "onZoom");
        assert_eq!(json["wide"]["thumbnailShape"], "crop");
        assert_eq!(serde_json::to_value(SortKey::ModifiedAt)?, "modified_at");
        Ok(())
    }
}
