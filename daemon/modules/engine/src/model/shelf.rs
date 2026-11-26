use serde::{Deserialize, Serialize};

use crate::prelude::*;

/// How a listing is laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DisplayLayout {
    Grid,
    List,
    Timeline,
}

/// The display values chosen by the width of the screen. An unset value is taken from the layer below.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayValueSet {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<DisplayLayout>,
    /// The size step of a tile, in the range of the display preferences.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tile_size: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thumbnail_shape: Option<ThumbnailShape>,
}

/// The display values a Shelf or the settings of a folder hold, in the shape of the display preferences with every value optional.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisplayValues {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wide: Option<DisplayValueSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrow: Option<DisplayValueSet>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sort: Option<SortPreference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_open: Option<ArchiveOpen>,
}

/// One start of a Shelf: a Library and a Library-relative path in normal form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelfStart {
    pub library_id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelfScope {
    pub starts: Vec<ShelfStart>,
    pub recursive: bool,
    pub inside_archives: bool,
}

/// The Filter of a Shelf, kept as it was given. Whether it can be listed is checked by whoever saves it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShelfFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rating: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kinds: Option<Vec<String>>,
}

/// The display values saved for one folder, named by its Library and its Library-relative path in normal form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderSetting {
    pub library_id: String,
    pub path: String,
    pub display: DisplayValues,
}

/// What a client saves as a Shelf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShelfInput {
    pub name: String,
    pub scope: ShelfScope,
    pub filter: ShelfFilter,
    pub display: DisplayValues,
}

/// A saved Shelf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shelf {
    pub id: String,
    pub name: String,
    pub scope: ShelfScope,
    pub filter: ShelfFilter,
    pub display: DisplayValues,
}

impl DisplayValues {
    /// Checks the values of folder settings, whose Layout cannot be the timeline.
    pub fn validate_for_folder(&self) -> Result<()> {
        if [self.wide, self.narrow].into_iter().flatten().any(|set| set.view == Some(DisplayLayout::Timeline)) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("the timeline layout can be saved only in a shelf"));
        }
        self.validate()
    }

    pub fn validate(&self) -> Result<()> {
        for (name, set) in [("wide", self.wide), ("narrow", self.narrow)] {
            if let Some(tile_size) = set.and_then(|set| set.tile_size)
                && !(DisplaySet::MIN_TILE_SIZE..=DisplaySet::MAX_TILE_SIZE).contains(&tile_size)
            {
                return Err(Error::new(ErrorKind::InvalidInput).with_message(format!(
                    "{name}.tileSize must be between {} and {}",
                    DisplaySet::MIN_TILE_SIZE,
                    DisplaySet::MAX_TILE_SIZE
                )));
            }
        }
        Ok(())
    }
}

impl ShelfInput {
    pub const MAX_NAME_LENGTH: usize = 100;

    /// Checks what can be checked without listing, and trims the name.
    pub fn normalized(&self) -> Result<Self> {
        let name = self.name.trim();
        if name.is_empty() || name.chars().count() > Self::MAX_NAME_LENGTH {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("name must have 1 to {} characters", Self::MAX_NAME_LENGTH)));
        }
        self.display.validate()?;
        Ok(Self {
            name: name.to_string(),
            ..self.clone()
        })
    }

    /// The default Shelf over `starts`: the timeline of the images and videos under them, including those inside archives.
    pub fn default_shelf(starts: &[ShelfStart]) -> Self {
        let set = DisplayValueSet {
            view: Some(DisplayLayout::Timeline),
            ..DisplayValueSet::default()
        };
        Self {
            name: "Timeline".to_string(),
            scope: ShelfScope {
                starts: starts.to_vec(),
                recursive: true,
                inside_archives: true,
            },
            filter: ShelfFilter {
                kinds: Some(vec!["image".to_string(), "video".to_string()]),
                ..ShelfFilter::default()
            },
            display: DisplayValues {
                wide: Some(set),
                narrow: Some(set),
                ..DisplayValues::default()
            },
        }
    }
}
