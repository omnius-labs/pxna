use std::path::Path;

use omnius_pxna_engine::service::{NestedFsEntry, ResolvedStoragePath};

use crate::{config::LibraryConfig, prelude::*, service::library::LibraryRelativePath};

/// How many starts a Scope has at most.
pub const MAX_STARTS: usize = 64;

/// One start of a Scope: a Library and a Library-relative path, as the client named it.
///
/// The path is kept as it was given, because the response names the starts that were cut off, walked, or missing by it.
/// Two starts are equal when their Library ids and the paths as given are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopeStart {
    library_id: String,
    given_path: String,
    path: LibraryRelativePath,
}

impl ScopeStart {
    /// A Library id that is not of the form a configured one has and a path that is not a Library-relative path are `InvalidInput`.
    /// Whether the Library is configured is not checked: a start in a Library that is not is a missing start.
    pub fn new(library_id: &str, path: &str) -> Result<Self> {
        if !LibraryConfig::is_valid_id(library_id) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("start has no valid library id: {library_id:?}")));
        }
        Ok(Self {
            library_id: library_id.to_string(),
            given_path: path.to_string(),
            path: LibraryRelativePath::parse(path)?,
        })
    }

    /// Splits `value` at its first `/` into the Library id and the path, which is empty when there is none, and keeps the path with its `/`.
    pub fn parse(value: &str) -> Result<Self> {
        let (library_id, path) = value.find('/').map_or((value, ""), |index| value.split_at(index));
        Self::new(library_id, path)
    }

    /// The text `parse` was given, which `parse` reads back as this start.
    pub fn value(&self) -> String {
        format!("{}{}", self.library_id, self.given_path)
    }

    pub fn library_id(&self) -> &str {
        &self.library_id
    }

    /// The path as it was given.
    pub fn given_path(&self) -> &str {
        &self.given_path
    }

    pub fn path(&self) -> &LibraryRelativePath {
        &self.path
    }
}

/// The starts of a listing, and whether it reads the descendants of the starts and the insides of archives.
#[derive(Debug, Clone)]
pub struct Scope {
    starts: Vec<ScopeStart>,
    recursive: bool,
    inside_archives: bool,
}

impl Scope {
    /// None and more than `MAX_STARTS` starts are `InvalidInput`.
    pub fn new(starts: Vec<ScopeStart>, recursive: bool, inside_archives: bool) -> Result<Self> {
        if starts.is_empty() || starts.len() > MAX_STARTS {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("a scope has 1-{MAX_STARTS} starts: {}", starts.len())));
        }
        Ok(Self {
            starts,
            recursive,
            inside_archives,
        })
    }

    /// More than `MAX_STARTS` starts, none, and a start that is not of the form of `ScopeStart::parse` are `InvalidInput`.
    pub fn parse(values: &[String], recursive: bool, inside_archives: bool) -> Result<Self> {
        Self::new(values.iter().map(|value| ScopeStart::parse(value)).collect::<Result<_>>()?, recursive, inside_archives)
    }

    pub fn starts(&self) -> &[ScopeStart] {
        &self.starts
    }

    pub fn recursive(&self) -> bool {
        self.recursive
    }

    pub fn inside_archives(&self) -> bool {
        self.inside_archives
    }
}

/// Where a resolved start is: its Library, its Library-relative path, and the archive files on the way to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartPlace {
    library_id: String,
    segments: Vec<String>,
    /// The Library-relative path of each archive file on the start's path, outermost first, the start itself included when it is an archive.
    archive_files: Vec<Vec<String>>,
}

impl StartPlace {
    pub fn new(library_id: &str, segments: &[String], archive_files: Vec<Vec<String>>) -> Self {
        Self {
            library_id: library_id.to_string(),
            segments: segments.to_vec(),
            archive_files,
        }
    }

    /// The archive files on a resolved path are the archive segments of its `NestedPath`: every segment but the last, each the path of an archive file
    /// inside the one before it, and the first a host path under the Library root.
    fn of_resolved(start: &ScopeStart, folder: &ResolvedStoragePath, library_root: &Path) -> Result<Self> {
        let segments = folder.path().iter().collect::<Vec<_>>();
        let mut archive_files = Vec::new();
        let mut current = Vec::<String>::new();
        for (index, segment) in segments.iter().take(segments.len().saturating_sub(1)).enumerate() {
            if index == 0 {
                let relative = Path::new(segment)
                    .strip_prefix(library_root)
                    .map_err(|_| Error::new(ErrorKind::UnexpectedError).with_message(format!("entry is outside of the library: {segment}")))?;
                current = relative.iter().map(|part| part.to_string_lossy().into_owned()).collect();
            } else {
                current.extend(segment.split('/').filter(|part| !part.is_empty()).map(str::to_string));
            }
            archive_files.push(current.clone());
        }
        Ok(Self::new(start.library_id(), start.path().segments(), archive_files))
    }

    /// Whether the Entries of this start are the Entries of `lower` too, so that `lower` adds nothing to a listing that has this start.
    ///
    /// A start covers itself, and a recursive one covers every start of its Library whose path is at or under its own, unless an archive boundary lies between them
    /// and the Scope does not read the insides of archives, since the upper start then lists that archive as one Entry. An archive boundary lies between a start
    /// and one under it when an archive file on the lower start's path, the lower start itself when it is an archive, is strictly under the upper start's path.
    pub fn covers(&self, lower: &Self, scope: &Scope) -> bool {
        if self.library_id != lower.library_id || !lower.segments.starts_with(&self.segments) {
            return false;
        }
        if self.segments == lower.segments {
            return true;
        }
        if !scope.recursive() {
            return false;
        }
        scope.inside_archives() || !lower.archive_files.iter().any(|file| file.len() > self.segments.len() && file.starts_with(&self.segments))
    }
}

/// A start that exists, with what the listing of it is built from.
pub struct ResolvedStart {
    start: ScopeStart,
    place: StartPlace,
    folder: ResolvedStoragePath,
    entry: NestedFsEntry,
}

impl ResolvedStart {
    pub fn new(start: ScopeStart, folder: ResolvedStoragePath, entry: NestedFsEntry, library_root: &Path) -> Result<Self> {
        Ok(Self {
            place: StartPlace::of_resolved(&start, &folder, library_root)?,
            start,
            folder,
            entry,
        })
    }

    pub fn start(&self) -> &ScopeStart {
        &self.start
    }

    pub fn place(&self) -> &StartPlace {
        &self.place
    }

    pub fn folder(&self) -> &ResolvedStoragePath {
        &self.folder
    }

    pub fn entry(&self) -> &NestedFsEntry {
        &self.entry
    }
}

/// What resolving one start came to.
pub enum StartResolution {
    Found(Box<ResolvedStart>),
    /// The start's folder or Library does not exist, or it names a file that is neither a folder nor an archive.
    Missing(ScopeStart),
}

/// The starts of a Scope that exist and add Entries, in the order they were given, and the starts that are missing.
pub struct ResolvedScope {
    starts: Vec<ResolvedStart>,
    missing: Vec<ScopeStart>,
}

impl ResolvedScope {
    /// Drops, among the resolved starts only, each start that another one covers, keeping the covering start at its own position and, of two identical starts, the first.
    /// A missing start covers nothing, and is named whatever covers its path.
    pub fn new(resolutions: Vec<StartResolution>, scope: &Scope) -> Self {
        let mut found = Vec::new();
        let mut missing = Vec::<ScopeStart>::new();
        for resolution in resolutions {
            match resolution {
                StartResolution::Found(start) => found.push(*start),
                StartResolution::Missing(start) if !missing.contains(&start) => missing.push(start),
                StartResolution::Missing(_) => {}
            }
        }
        let places = found.iter().map(|start| start.place().clone()).collect::<Vec<_>>();
        let kept = (0..places.len())
            .map(|index| {
                !places
                    .iter()
                    .enumerate()
                    .any(|(other, place)| other != index && place.covers(&places[index], scope) && (other < index || !places[index].covers(place, scope)))
            })
            .collect::<Vec<_>>();
        let starts = found.into_iter().zip(kept).filter_map(|(start, kept)| kept.then_some(start)).collect();
        Self { starts, missing }
    }

    pub fn starts(&self) -> &[ResolvedStart] {
        &self.starts
    }

    /// The missing starts as they were given, in the order they were given, one of identical starts.
    pub fn missing(&self) -> &[ScopeStart] {
        &self.missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(value: &str) -> Result<ScopeStart> {
        ScopeStart::parse(value)
    }

    fn scope(recursive: bool, inside_archives: bool) -> Scope {
        Scope {
            starts: Vec::new(),
            recursive,
            inside_archives,
        }
    }

    fn segments(path: &str) -> Vec<String> {
        path.split('/').filter(|part| !part.is_empty()).map(str::to_string).collect()
    }

    /// The place of a start at `path` of `library`, with the Library-relative paths of the archive files on its way.
    fn place(library: &str, path: &str, archive_files: &[&str]) -> StartPlace {
        StartPlace::new(library, &segments(path), archive_files.iter().map(|file| segments(file)).collect())
    }

    #[test]
    fn a_start_value_is_a_library_id_and_a_path_test() -> testresult::TestResult {
        let start = parse("photos/2025/trip")?;
        assert_eq!(
            (start.library_id(), start.path().segments(), start.value().as_str()),
            ("photos", &["2025".to_string(), "trip".to_string()][..], "photos/2025/trip")
        );
        let root = parse("photos")?;
        assert_eq!((root.library_id(), root.path().segments().len()), ("photos", 0));
        // The trailing slash and `.` are not significant, `..` cancels a segment, and the value is kept as it was given.
        let unclean = parse("photos/a/../b/./")?;
        assert_eq!(unclean.path().segments(), ["b".to_string()]);
        assert_eq!(unclean.value(), "photos/a/../b/./");
        assert_eq!(parse("photos/")?.path().segments().len(), 0);
        // A path inside an archive is an ordinary path to the parser.
        assert_eq!(parse("photos/pack.7z/dir")?.path().segments(), ["pack.7z".to_string(), "dir".to_string()]);
        Ok(())
    }

    #[test]
    fn a_malformed_start_value_is_invalid_input_test() {
        for value in ["", "/photos", "/", "photos/../..", "photos/a\\b", "photos/C:/x", "photos/a\0b"] {
            assert_eq!(*parse(value).unwrap_err().kind(), ErrorKind::InvalidInput, "{value:?}");
        }
    }

    #[test]
    fn a_start_is_built_from_a_library_id_and_a_path_kept_as_given_test() -> testresult::TestResult {
        let start = ScopeStart::new("photos", "/a/../b/./")?;
        assert_eq!(
            (start.library_id(), start.given_path(), start.path().segments()),
            ("photos", "/a/../b/./", ["b".to_string()].as_slice())
        );
        assert_eq!(start.value(), "photos/a/../b/./");
        // A path with or without its leading `/`, and the root as `/` or as nothing, are the same place and different starts as given.
        assert_eq!(ScopeStart::new("photos", "/")?.path().segments(), ScopeStart::new("photos", "")?.path().segments());
        assert_ne!(ScopeStart::new("photos", "/x")?, ScopeStart::new("photos", "/x/")?);
        assert_ne!(ScopeStart::new("photos", "/x")?, ScopeStart::new("videos", "/x")?);
        assert_eq!(ScopeStart::new("photos", "/x")?, ScopeStart::new("photos", "/x")?);
        // `parse` reads back what `value` gives.
        for value in ["photos", "photos/", "photos/x", "photos/a/../b/./"] {
            assert_eq!(parse(value)?.value(), value);
        }
        Ok(())
    }

    #[test]
    fn a_start_with_an_invalid_library_id_or_path_is_invalid_input_test() {
        let long = "a".repeat(65);
        for library_id in ["", ".", "..", "a b", "a/b", "ph\"otos", "フォト", long.as_str()] {
            assert_eq!(*ScopeStart::new(library_id, "/").unwrap_err().kind(), ErrorKind::InvalidInput, "{library_id:?}");
        }
        for path in ["/..", "/a/../..", "/a\\b", "/C:/x", "/a\0b"] {
            assert_eq!(*ScopeStart::new("photos", path).unwrap_err().kind(), ErrorKind::InvalidInput, "{path:?}");
        }
        assert!(ScopeStart::new(&"a".repeat(64), "/").is_ok());
        assert!(ScopeStart::new("A.b_c-1", "/").is_ok());
        // A Library that is not configured is not a malformed start.
        assert!(ScopeStart::new("nolibrary", "/x").is_ok());
    }

    #[test]
    fn a_scope_of_starts_has_one_to_sixty_four_test() {
        let starts = |count: usize| (0..count).map(|number| ScopeStart::new("photos", &format!("/{number}")).unwrap()).collect::<Vec<_>>();
        assert_eq!(Scope::new(starts(1), true, false).unwrap().starts().len(), 1);
        assert_eq!(Scope::new(starts(64), true, false).unwrap().starts().len(), 64);
        assert_eq!(*Scope::new(starts(65), true, false).unwrap_err().kind(), ErrorKind::InvalidInput);
        assert_eq!(*Scope::new(Vec::new(), true, false).unwrap_err().kind(), ErrorKind::InvalidInput);
    }

    #[test]
    fn a_missing_start_given_twice_is_named_once_as_given_test() {
        let missing = |value: &str| StartResolution::Missing(parse(value).unwrap());
        let scope = Scope::new(vec![parse("photos").unwrap()], true, false).unwrap();
        let resolved = ResolvedScope::new(
            vec![
                missing("photos/gone"),
                missing("nolibrary/x"),
                missing("photos/gone"),
                missing("photos/gone/"),
                missing("photos/x/../gone"),
            ],
            &scope,
        );
        assert_eq!(
            resolved.missing().iter().map(ScopeStart::value).collect::<Vec<_>>(),
            ["photos/gone", "nolibrary/x", "photos/gone/", "photos/x/../gone"],
            "identical starts once, the others as they were given"
        );
        assert!(resolved.starts().is_empty());
    }

    #[test]
    fn a_scope_has_one_to_sixty_four_starts_test() {
        let values = |count: usize| (0..count).map(|number| format!("photos/{number}")).collect::<Vec<_>>();
        assert_eq!(Scope::parse(&values(1), true, false).unwrap().starts().len(), 1);
        assert_eq!(Scope::parse(&values(64), true, false).unwrap().starts().len(), 64);
        assert_eq!(*Scope::parse(&values(65), true, false).unwrap_err().kind(), ErrorKind::InvalidInput);
        assert_eq!(*Scope::parse(&[], true, false).unwrap_err().kind(), ErrorKind::InvalidInput);
        assert_eq!(
            *Scope::parse(&["photos".to_string(), "".to_string()], true, false).unwrap_err().kind(),
            ErrorKind::InvalidInput
        );
        let scope = Scope::parse(&["b/x".to_string(), "a".to_string()], true, true).unwrap();
        assert_eq!(
            scope.starts().iter().map(ScopeStart::value).collect::<Vec<_>>(),
            ["b/x", "a"],
            "the order is the order given"
        );
        assert!(scope.recursive() && scope.inside_archives());
    }

    #[test]
    fn a_start_covers_itself_and_the_starts_under_it_test() {
        let recursive = scope(true, false);
        let upper = place("a", "/x", &[]);
        assert!(upper.covers(&upper, &recursive));
        assert!(upper.covers(&place("a", "/x/y", &[]), &recursive));
        assert!(upper.covers(&place("a", "/x/y/z", &[]), &recursive));
        assert!(!upper.covers(&place("a", "/xy", &[]), &recursive), "/xy is not under /x");
        assert!(!upper.covers(&place("a", "/", &[]), &recursive), "a start does not cover the one above it");
        assert!(!upper.covers(&place("a", "/other", &[]), &recursive));
        assert!(!upper.covers(&place("b", "/x/y", &[]), &recursive), "another Library is another start");
        assert!(place("a", "/", &[]).covers(&place("a", "/x/y", &[]), &recursive));
        assert!(place("a", "/", &[]).covers(&place("a", "/", &[]), &recursive));
    }

    #[test]
    fn a_start_of_a_listing_of_direct_children_covers_only_itself_test() {
        let flat = scope(false, false);
        let upper = place("a", "/x", &[]);
        assert!(upper.covers(&upper, &flat));
        assert!(!upper.covers(&place("a", "/x/y", &[]), &flat));
        let flat_with_insides = scope(false, true);
        assert!(!upper.covers(&place("a", "/x/y", &[]), &flat_with_insides));
    }

    #[test]
    fn an_archive_boundary_stops_a_start_from_covering_without_the_insides_test() {
        let archive_start = place("a", "/pack.7z", &["/pack.7z"]);
        let inside = place("a", "/pack.7z/dir", &["/pack.7z"]);
        let nested = place("a", "/pack.7z/nested.7z/dir", &["/pack.7z", "/pack.7z/nested.7z"]);
        let root = place("a", "/", &[]);
        let without = scope(true, false);
        let with = scope(true, true);

        // The root lists the archive as one Entry without the insides, so a start that is the archive or is inside it adds what the root does not list.
        assert!(!root.covers(&archive_start, &without));
        assert!(!root.covers(&inside, &without));
        assert!(!root.covers(&nested, &without));
        assert!(root.covers(&archive_start, &with));
        assert!(root.covers(&inside, &with));
        assert!(root.covers(&nested, &with));

        // The same archive twice is the same start, and a start inside the archive below the archive start has no boundary between them.
        assert!(archive_start.covers(&archive_start, &without));
        assert!(archive_start.covers(&inside, &without));
        assert!(archive_start.covers(&inside, &with));
        // A nested archive is a boundary under the archive start.
        assert!(!archive_start.covers(&nested, &without));
        assert!(archive_start.covers(&nested, &with));
        assert!(inside.covers(&place("a", "/pack.7z/dir/deeper", &["/pack.7z"]), &without));
        // A folder start below a folder start has no boundary even when an archive is elsewhere in the Library.
        assert!(place("a", "/x", &[]).covers(&place("a", "/x/y", &[]), &without));
    }
}
