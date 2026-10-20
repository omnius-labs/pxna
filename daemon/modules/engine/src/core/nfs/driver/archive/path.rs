use crate::{model::FsEntry, prelude::*};

use super::ArchiveError;

pub(super) struct ArchiveEntryPath;

impl ArchiveEntryPath {
    pub(super) fn normalize(raw: &str, is_directory: bool) -> Result<String> {
        let replaced = raw.replace('\\', "/");
        let mut normalized = replaced.as_str();
        while let Some(rest) = normalized.strip_prefix("./") {
            normalized = rest;
        }
        if is_directory {
            normalized = normalized.strip_suffix('/').unwrap_or(normalized);
        }

        if normalized.as_bytes().contains(&0) {
            return Err(ArchiveError::invalid_format(format!("archive entry path contains NUL: {raw:?}")));
        }
        if normalized.is_empty() {
            return Err(ArchiveError::invalid_format(format!("archive entry path is empty after normalization: {raw:?}")));
        }
        if normalized.starts_with('/') || Self::has_drive_prefix(normalized) {
            return Err(ArchiveError::invalid_format(format!("archive entry path is absolute: {raw:?}")));
        }
        for segment in normalized.split('/') {
            if segment.is_empty() || segment == "." || segment == ".." {
                return Err(ArchiveError::invalid_format(format!("archive entry path has an invalid segment: {raw:?}")));
            }
        }
        if normalized.len() > FsEntry::MAX_PATH_LENGTH {
            return Err(ArchiveError::limit_exceeded(format!(
                "archive entry path exceeds {} bytes: {raw:?}",
                FsEntry::MAX_PATH_LENGTH
            )));
        }
        Ok(normalized.to_string())
    }

    fn has_drive_prefix(path: &str) -> bool {
        let bytes = path.as_bytes();
        bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':'
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_slashes_prefix_and_directory_suffix_test() {
        assert_eq!(ArchiveEntryPath::normalize("././a\\b/", true).unwrap(), "a/b");
        assert_eq!(ArchiveEntryPath::normalize("a/b", false).unwrap(), "a/b");
    }

    #[test]
    fn rejects_invalid_paths_test() {
        for path in ["/a", "C:/a", "a//b", "a/./b", "a/../b", "a\0b", "./", "a//"] {
            assert_eq!(ArchiveEntryPath::normalize(path, path == "./").unwrap_err().kind(), &ErrorKind::InvalidFormat, "{path:?}");
        }
        let too_long = "a".repeat(FsEntry::MAX_PATH_LENGTH + 1);
        assert_eq!(ArchiveEntryPath::normalize(&too_long, false).unwrap_err().kind(), &ErrorKind::LimitExceeded);
    }
}
