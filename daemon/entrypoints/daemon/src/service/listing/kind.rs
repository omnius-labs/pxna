use omnius_pxna_engine::service::MediaKind;
use omnius_pxna_interface::{models, types::Nullable};

use crate::prelude::*;

/// The kind of an Entry that a Filter chooses by, as the name index holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntryKind {
    Folder,
    Archive,
    Image,
    Video,
    /// Any other file, and a symbolic link whatever its name.
    Other,
}

impl EntryKind {
    pub const ALL: [Self; 5] = [Self::Folder, Self::Archive, Self::Image, Self::Video, Self::Other];

    /// The kind named by a `kind` value of the request.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "folder" => Ok(Self::Folder),
            "archive" => Ok(Self::Archive),
            "image" => Ok(Self::Image),
            "video" => Ok(Self::Video),
            "other" => Ok(Self::Other),
            _ => Err(Error::new(ErrorKind::InvalidInput).with_message(format!("kind must be folder, archive, image, video or other: {value}"))),
        }
    }

    /// A folder, an archive (a file whose `archive` is true), an image or a video (by name, for a file that is not an archive), or other.
    pub fn of(entry: &models::FileSystemEntry) -> Self {
        match entry.kind {
            models::FileSystemEntryKind::Folder => Self::Folder,
            models::FileSystemEntryKind::Link => Self::Other,
            models::FileSystemEntryKind::File => {
                if matches!(&entry.file, Some(Nullable::Present(file)) if file.archive) {
                    return Self::Archive;
                }
                match MediaKind::of_name(&entry.name) {
                    Some(MediaKind::Video) => Self::Video,
                    Some(MediaKind::Image | MediaKind::FfmpegImage | MediaKind::HeifImage) => Self::Image,
                    None => Self::Other,
                }
            }
        }
    }

    fn bit(self) -> u8 {
        match self {
            Self::Folder => 1,
            Self::Archive => 2,
            Self::Image => 4,
            Self::Video => 8,
            Self::Other => 16,
        }
    }
}

/// A set of kinds. A Filter that names no kind holds the empty set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct EntryKinds(u8);

impl EntryKinds {
    pub fn all() -> Self {
        Self::of(EntryKind::ALL)
    }

    pub fn of(kinds: impl IntoIterator<Item = EntryKind>) -> Self {
        Self(kinds.into_iter().fold(0, |bits, kind| bits | kind.bit()))
    }

    /// The kinds that can have a rating or a tag: images and videos.
    pub fn media() -> Self {
        Self::of([EntryKind::Image, EntryKind::Video])
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn contains(self, kind: EntryKind) -> bool {
        self.0 & kind.bit() != 0
    }

    /// The kinds in both sets.
    pub fn intersect(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }

    /// The kinds in the set, in the order of `EntryKind::ALL`.
    pub fn iter(self) -> impl Iterator<Item = EntryKind> {
        EntryKind::ALL.into_iter().filter(move |kind| self.contains(*kind))
    }

    pub fn without(self, kind: EntryKind) -> Self {
        Self(self.0 & !kind.bit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::service::listing::tests::{entry, file, folder};

    fn link(name: &str) -> models::FileSystemEntry {
        let mut e = entry(name);
        e.kind = models::FileSystemEntryKind::Link;
        e
    }

    #[test]
    fn classifies_each_kind_test() {
        assert_eq!(EntryKind::of(&folder("dir")), EntryKind::Folder);
        assert_eq!(EntryKind::of(&file("pack.zip", true)), EntryKind::Archive);
        assert_eq!(EntryKind::of(&file("a.PNG", false)), EntryKind::Image);
        assert_eq!(EntryKind::of(&file("a.heic", false)), EntryKind::Image);
        assert_eq!(EntryKind::of(&file("a.avif", false)), EntryKind::Image);
        assert_eq!(EntryKind::of(&file("clip.mp4", false)), EntryKind::Video);
        assert_eq!(EntryKind::of(&file("notes.txt", false)), EntryKind::Other);
        assert_eq!(EntryKind::of(&entry("noextension")), EntryKind::Other);
        // An archive is an archive even when its name is a media name, and a link is other whatever its name.
        assert_eq!(EntryKind::of(&file("odd.png", true)), EntryKind::Archive);
        assert_eq!(EntryKind::of(&link("shortcut.png")), EntryKind::Other);
        assert_eq!(EntryKind::of(&link("shortcut")), EntryKind::Other);
    }

    #[test]
    fn a_set_holds_the_kinds_it_was_made_of_test() {
        let set = EntryKinds::of([EntryKind::Image, EntryKind::Other]);
        assert!(set.contains(EntryKind::Image) && set.contains(EntryKind::Other));
        assert!(!set.contains(EntryKind::Folder) && !set.contains(EntryKind::Video) && !set.contains(EntryKind::Archive));
        assert!(EntryKinds::default().is_empty());
        assert_eq!(set.intersect(EntryKinds::media()), EntryKinds::of([EntryKind::Image]));
        assert!(EntryKinds::media().intersect(EntryKinds::of([EntryKind::Folder, EntryKind::Other])).is_empty());
        assert!(EntryKind::ALL.iter().all(|kind| EntryKinds::all().contains(*kind)));
        assert!(!EntryKinds::all().without(EntryKind::Folder).contains(EntryKind::Folder));
        assert!(EntryKinds::all().without(EntryKind::Folder).contains(EntryKind::Archive));
    }
}
