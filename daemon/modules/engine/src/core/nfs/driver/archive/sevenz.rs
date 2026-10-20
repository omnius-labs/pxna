use std::{io::Write as _, path::Path, sync::Arc};

use sevenz_rust2::{Archive, BlockDecoder, EncoderMethod, Password};

use crate::prelude::*;

use super::{
    ArchiveError,
    index::{ArchiveBlockId, ArchiveEntry, ArchiveIndex},
    options::ArchiveOptions,
    path::ArchiveEntryPath,
};

const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
const FILE_ATTRIBUTE_UNIX_EXTENSION: u32 = 0x8000;
const UNIX_FILE_TYPE_MASK: u32 = 0o170000;
const UNIX_REGULAR_FILE: u32 = 0o100000;
const UNIX_DIRECTORY: u32 = 0o040000;
const UNIX_SYMLINK: u32 = 0o120000;

#[derive(Clone)]
pub(super) struct SevenZipArchive {
    archive: Arc<Archive>,
    #[cfg(test)]
    decode_count: Arc<std::sync::atomic::AtomicUsize>,
    #[cfg(test)]
    decoded_bytes: Arc<std::sync::atomic::AtomicU64>,
}

impl SevenZipArchive {
    pub fn inspect(path: &Path, options: &ArchiveOptions) -> Result<(Self, ArchiveIndex)> {
        let archive = Archive::open(path).map_err(Self::map_error)?;
        if archive.files.len() > options.max_entry_count {
            return Err(ArchiveError::limit_exceeded(format!("archive has more than {} entries", options.max_entry_count)));
        }
        if archive
            .blocks
            .iter()
            .flat_map(|block| &block.coders)
            .any(|coder| coder.encoder_method_id() == EncoderMethod::ID_AES256_SHA256)
        {
            return Err(Error::new(ErrorKind::PasswordRequired).with_message("7z archive is encrypted"));
        }

        let mut entries = Vec::with_capacity(archive.files.len());
        for (file_index, file) in archive.files.iter().enumerate() {
            if file.is_anti_item {
                return Err(ArchiveError::invalid_format(format!("7z anti-item is unsupported: {}", file.name)));
            }
            let kind = Self::classify(file)?;
            let normalized = ArchiveEntryPath::normalize(&file.name, kind == FsEntryKind::Dir)?;
            let modified = file.has_last_modified_date.then(|| std::time::SystemTime::from(file.last_modified_date).into());
            let block_id = archive.stream_map.file_block_index.get(file_index).copied().flatten().map(ArchiveBlockId::SevenZip);
            entries.push(ArchiveEntry::new(normalized, kind, file.size, modified, block_id));
        }
        let index = ArchiveIndex::build(entries, options.max_entry_count)?;
        Ok((
            Self {
                archive: Arc::new(archive),
                #[cfg(test)]
                decode_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
                #[cfg(test)]
                decoded_bytes: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            },
            index,
        ))
    }

    pub fn block_size(&self, block_index: usize) -> Result<u64> {
        let mut total = 0_u64;
        for (file_index, mapped) in self.archive.stream_map.file_block_index.iter().enumerate() {
            if *mapped == Some(block_index) {
                total = total
                    .checked_add(self.archive.files[file_index].size)
                    .ok_or_else(|| ArchiveError::limit_exceeded("7z block output size overflow"))?;
            }
        }
        Ok(total)
    }

    /// block を 1 回 decode し、block に含まれる全 file を `staging_dir` 配下の一時 file として返す。
    pub fn extract(&self, source_path: &Path, block_index: usize, staging_dir: &Path) -> Result<Vec<(String, tempfile::NamedTempFile)>> {
        #[cfg(test)]
        self.decode_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if block_index >= self.archive.blocks.len() {
            return Err(ArchiveError::invalid_format(format!("7z block index is out of range: {block_index}")));
        }
        let mut source = std::fs::File::open(source_path)?;
        let password = Password::empty();
        let decoder = BlockDecoder::new(1, block_index, &self.archive, &password, &mut source);
        let mut outputs = Vec::new();
        decoder
            .for_each_entries(&mut |file, reader| {
                if !file.has_stream || file.is_directory {
                    return Ok(true);
                }
                let kind = Self::classify(file).map_err(Self::to_callback_error)?;
                let path = ArchiveEntryPath::normalize(&file.name, kind == FsEntryKind::Dir).map_err(Self::to_callback_error)?;
                let mut output = tempfile::NamedTempFile::new_in(staging_dir)?;
                let _copied = std::io::copy(reader, output.as_file_mut())?;
                #[cfg(test)]
                self.decoded_bytes.fetch_add(_copied, std::sync::atomic::Ordering::SeqCst);
                output.as_file_mut().flush()?;
                output.as_file().sync_all()?;
                outputs.push((path, output));
                Ok(true)
            })
            .map_err(Self::map_error)?;

        Ok(outputs)
    }

    #[cfg(test)]
    pub fn decode_count(&self) -> usize {
        self.decode_count.load(std::sync::atomic::Ordering::SeqCst)
    }

    #[cfg(test)]
    pub fn decoded_bytes(&self) -> u64 {
        self.decoded_bytes.load(std::sync::atomic::Ordering::SeqCst)
    }
}

impl SevenZipArchive {
    fn classify(entry: &sevenz_rust2::ArchiveEntry) -> Result<FsEntryKind> {
        let attributes = if entry.has_windows_attributes { entry.windows_attributes } else { 0 };
        let reparse = attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0;
        let posix_mode = if attributes & FILE_ATTRIBUTE_UNIX_EXTENSION != 0 || attributes >> 16 != 0 {
            attributes >> 16
        } else {
            0
        };
        let file_type = posix_mode & UNIX_FILE_TYPE_MASK;

        if reparse || file_type == UNIX_SYMLINK {
            if entry.is_directory {
                return Err(ArchiveError::invalid_format(format!("7z symlink is also marked as directory: {}", entry.name)));
            }
            return Ok(FsEntryKind::Link);
        }
        match file_type {
            UNIX_DIRECTORY if !entry.is_directory => Err(ArchiveError::invalid_format(format!("7z directory attributes conflict with entry kind: {}", entry.name))),
            UNIX_REGULAR_FILE if entry.is_directory => Err(ArchiveError::invalid_format(format!("7z file attributes conflict with entry kind: {}", entry.name))),
            UNIX_DIRECTORY | UNIX_REGULAR_FILE | 0 => Ok(if entry.is_directory { FsEntryKind::Dir } else { FsEntryKind::File }),
            _ => Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("unsupported 7z entry type: {}", entry.name))),
        }
    }

    fn to_callback_error(error: Error) -> sevenz_rust2::Error {
        sevenz_rust2::Error::from(std::io::Error::other(error))
    }

    fn map_error(error: sevenz_rust2::Error) -> Error {
        match error {
            error @ (sevenz_rust2::Error::PasswordRequired | sevenz_rust2::Error::MaybeBadPassword(_)) => {
                Error::from_error(error, ErrorKind::PasswordRequired).with_message("7z archive is encrypted")
            }
            error @ (sevenz_rust2::Error::UnsupportedCompressionMethod(_) | sevenz_rust2::Error::ExternalUnsupported | sevenz_rust2::Error::Unsupported(_)) => {
                Error::from_error(error, ErrorKind::UnsupportedType)
            }
            sevenz_rust2::Error::Io(error, _) if error.kind() == std::io::ErrorKind::UnexpectedEof => ArchiveError::invalid_format_from(error),
            sevenz_rust2::Error::Io(error, _) | sevenz_rust2::Error::FileOpen(error, _) => Error::from(error),
            error @ sevenz_rust2::Error::MaxMemLimited { .. } => ArchiveError::limit_exceeded_from(error),
            error => ArchiveError::invalid_format_from(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_symlink_and_special_entry_test() {
        let mut link = sevenz_rust2::ArchiveEntry::new_file("link");
        link.has_windows_attributes = true;
        link.windows_attributes = FILE_ATTRIBUTE_UNIX_EXTENSION | ((UNIX_SYMLINK | 0o777) << 16);
        assert_eq!(SevenZipArchive::classify(&link).unwrap(), FsEntryKind::Link);

        let mut special = sevenz_rust2::ArchiveEntry::new_file("fifo");
        special.has_windows_attributes = true;
        special.windows_attributes = FILE_ATTRIBUTE_UNIX_EXTENSION | (0o010000 << 16);
        assert_eq!(SevenZipArchive::classify(&special).unwrap_err().kind(), &ErrorKind::UnsupportedType);
    }

    #[test]
    fn adapter_keeps_sevenz_error_as_source_test() {
        let error = SevenZipArchive::map_error(sevenz_rust2::Error::Unsupported("test".into()));

        assert_eq!(error.kind(), &ErrorKind::UnsupportedType);
        assert!(std::error::Error::source(&error).is_some());
    }
}
