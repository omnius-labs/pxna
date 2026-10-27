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
    /// The size level of the viewer's filmstrip (1 to 5). The narrow set keeps it but does not use it.
    pub filmstrip_size: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SortPreference {
    pub key: SortKey,
    pub order: SortOrder,
}

/// オーナーが選んでサーバに保存する、web の見た目と振る舞いの値の組。
///
/// 広い画面と狭い画面の 2 組と、画面の幅に関係しない並べ替えとタイムラインに並べる Library を持つ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    pub wide: DisplaySet,
    pub narrow: DisplaySet,
    pub sort: SortPreference,
    /// タイムラインに並べる Library の id。`None` はすべての Library である。
    ///
    /// 設定から消えた Library の id も保存したまま持ち、web が読み飛ばす。
    #[serde(rename = "timelineLibraries", default, skip_serializing_if = "Option::is_none")]
    pub timeline_libraries: Option<Vec<String>>,
}

impl DisplaySet {
    pub const MIN_TILE_SIZE: u8 = 1;
    pub const MAX_TILE_SIZE: u8 = 5;
    pub const MIN_FILMSTRIP_SIZE: u8 = 1;
    pub const MAX_FILMSTRIP_SIZE: u8 = 5;

    fn validate(&self, name: &str) -> Result<()> {
        if !(Self::MIN_TILE_SIZE..=Self::MAX_TILE_SIZE).contains(&self.tile_size) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("{name}.tileSize must be between {} and {}", Self::MIN_TILE_SIZE, Self::MAX_TILE_SIZE)));
        }
        if !(Self::MIN_FILMSTRIP_SIZE..=Self::MAX_FILMSTRIP_SIZE).contains(&self.filmstrip_size) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!(
                "{name}.filmstripSize must be between {} and {}",
                Self::MIN_FILMSTRIP_SIZE,
                Self::MAX_FILMSTRIP_SIZE
            )));
        }
        Ok(())
    }
}

impl Preferences {
    pub const MAX_TIMELINE_LIBRARIES: usize = 64;
    const MAX_LIBRARY_ID_LENGTH: usize = 64;

    /// 値の範囲を確かめる。列挙の値は、読み込みの時点で既知の値に限られる。
    pub fn validate(&self) -> Result<()> {
        self.wide.validate("wide")?;
        self.narrow.validate("narrow")?;
        if let Some(libraries) = &self.timeline_libraries {
            if libraries.is_empty() || libraries.len() > Self::MAX_TIMELINE_LIBRARIES {
                return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("timelineLibraries must have 1 to {} ids", Self::MAX_TIMELINE_LIBRARIES)));
            }
            for id in libraries {
                let valid = !id.is_empty() && id.len() <= Self::MAX_LIBRARY_ID_LENGTH && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
                if !valid {
                    return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("timelineLibraries has an invalid library id: {id:?}")));
                }
            }
        }
        Ok(())
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
            filmstrip_size: 3,
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
            timeline_libraries: None,
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
        assert_eq!((preferences.wide.filmstrip_size, preferences.narrow.filmstrip_size), (3, 3));
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
    fn filmstrip_size_accepts_the_range_and_rejects_outside_it_test() {
        for filmstrip_size in [1, 5] {
            let mut preferences = Preferences::default();
            preferences.wide.filmstrip_size = filmstrip_size;
            preferences.narrow.filmstrip_size = filmstrip_size;
            assert!(preferences.validate().is_ok(), "{filmstrip_size}");
        }
        for filmstrip_size in [0, 6, 255] {
            for narrow in [false, true] {
                let mut preferences = Preferences::default();
                let set = if narrow { &mut preferences.narrow } else { &mut preferences.wide };
                set.filmstrip_size = filmstrip_size;
                let error = preferences.validate().unwrap_err();
                assert_eq!(*error.kind(), ErrorKind::InvalidInput, "{filmstrip_size} {narrow}");
            }
        }
    }

    #[test]
    fn rejects_invalid_timeline_libraries_test() {
        for libraries in [vec![], vec!["a/b".to_string()], vec![String::new()], vec!["x".to_string(); 65]] {
            let preferences = Preferences {
                timeline_libraries: Some(libraries.clone()),
                ..Preferences::default()
            };
            assert_eq!(*preferences.validate().unwrap_err().kind(), ErrorKind::InvalidInput, "{libraries:?}");
        }
        let preferences = Preferences {
            timeline_libraries: Some(vec!["photos".to_string(), "gone.lib".to_string()]),
            ..Preferences::default()
        };
        assert!(preferences.validate().is_ok());
    }

    #[test]
    fn reads_values_saved_before_timeline_libraries_test() -> testresult::TestResult {
        let mut json = serde_json::to_value(Preferences::default())?;
        assert!(json.get("timelineLibraries").is_none(), "None は書かない");
        json.as_object_mut().unwrap().remove("timelineLibraries");
        assert_eq!(serde_json::from_value::<Preferences>(json)?.timeline_libraries, None);
        Ok(())
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
