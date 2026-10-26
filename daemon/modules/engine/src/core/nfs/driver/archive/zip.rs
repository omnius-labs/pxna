use std::{io::Write as _, path::Path, sync::Arc};

use async_zip::{
    base::read1::seek::{ZipArchiveInner, ZipArchiveReader},
    error::ZipError,
    spec::constructs::CDR,
};
use chrono::{DateTime, LocalResult, TimeZone as _, Utc};
use crc::{CRC_32_ISO_HDLC, Crc};
use encoding_rs::SHIFT_JIS;
use futures::AsyncReadExt as _;
use tokio::io::{AsyncReadExt as _, AsyncSeekExt as _, BufReader};
use tokio_util::compat::{Compat, TokioAsyncReadCompatExt as _};

use crate::prelude::*;

use super::{
    ArchiveError,
    index::{ArchiveBlockId, ArchiveEntry, ArchiveIndex},
    options::{ArchiveOptions, ZipLegacyEncoding},
    path::ArchiveEntryPath,
};

const UNIX_FILE_TYPE_MASK: u16 = 0o170000;
const UNIX_REGULAR_FILE: u16 = 0o100000;
const UNIX_DIRECTORY: u16 = 0o040000;
const UNIX_SYMLINK: u16 = 0o120000;
/// The upper byte of "version made by" for entries created on Unix.
const UNIX_HOST: u16 = 3;

type ZipFileSource = Compat<BufReader<tokio::fs::File>>;

#[derive(Clone)]
pub(super) struct ZipArchive {
    archive: Arc<ZipArchiveInner>,
}

impl ZipArchive {
    pub async fn inspect(path: &Path, options: &ArchiveOptions) -> Result<(Self, ArchiveIndex)> {
        let reader = ZipArchiveReader::open(Self::open_source(path).await?).await.map_err(Self::map_error)?;
        let archive = reader.inner().clone();
        let stored_entries = archive.cdrs();
        if stored_entries.len() > options.max_entry_count {
            return Err(ArchiveError::limit_exceeded(format!("archive has more than {} entries", options.max_entry_count)));
        }
        Self::reject_encrypted_entries(path, stored_entries).await?;

        let mut entries = Vec::with_capacity(stored_entries.len());
        for (ordinal, stored) in stored_entries.iter().enumerate() {
            let decoded = Self::decode_name(stored.insecure_file_name.as_bytes(), options.zip_legacy_encoding)?;
            let name_is_directory = decoded.ends_with('/');
            let kind = Self::classify(Self::unix_permissions(stored), name_is_directory, &decoded)?;
            let normalized = ArchiveEntryPath::normalize(&decoded, kind == FsEntryKind::Dir)?;
            let size = stored.uncompressed_size().map_err(Self::map_error)?;
            let block_id = if kind == FsEntryKind::Dir { None } else { Some(ArchiveBlockId::Zip(ordinal)) };
            entries.push(ArchiveEntry::new(normalized, kind, size, Self::modified(stored), block_id));
        }
        let index = ArchiveIndex::build(entries, options.max_entry_count)?;
        Ok((Self { archive }, index))
    }

    /// entry を `staging_dir` 配下の一時 file へ取り出し、size と CRC を検証する。
    pub fn extract(&self, source_path: &Path, ordinal: usize, expected_size: u64, staging_dir: &Path) -> Result<tempfile::NamedTempFile> {
        let archive = self.archive.clone();
        let source_path = source_path.to_path_buf();
        let staging_dir = staging_dir.to_path_buf();
        let runtime = tokio::runtime::Builder::new_current_thread().enable_io().build()?;
        runtime.block_on(async move {
            let expected_crc = archive
                .cdrs()
                .get(ordinal)
                .ok_or_else(|| ArchiveError::invalid_format(format!("ZIP entry {ordinal} does not exist")))?
                .cdrh
                .crc;
            let mut reader = ZipArchiveReader::new_with_inner(Self::open_source(&source_path).await?, archive);
            let mut entry = reader.file(ordinal).await.map_err(Self::map_error)?;
            let crc = Crc::<u32>::new(&CRC_32_ISO_HDLC);
            let mut digest = crc.digest();
            let mut output = tempfile::NamedTempFile::new_in(&staging_dir)?;
            let mut total = 0_u64;
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let read = entry.read(&mut buffer).await.map_err(|error| ArchiveError::source_path(source_path.clone(), error))?;
                if read == 0 {
                    break;
                }
                total = total.checked_add(read as u64).ok_or_else(|| ArchiveError::limit_exceeded("ZIP output size overflow"))?;
                if total > expected_size {
                    return Err(ArchiveError::invalid_format("ZIP entry produced more data than declared"));
                }
                digest.update(&buffer[..read]);
                output.write_all(&buffer[..read])?;
            }
            if total != expected_size {
                return Err(ArchiveError::invalid_format(format!("ZIP entry size mismatch: expected {expected_size}, got {total}")));
            }
            if digest.finalize() != expected_crc {
                return Err(ArchiveError::invalid_format("ZIP entry CRC mismatch"));
            }
            output.as_file_mut().sync_all()?;
            Ok(output)
        })
    }
}

impl ZipArchive {
    async fn open_source(path: &Path) -> Result<ZipFileSource> {
        Ok(BufReader::new(tokio::fs::File::open(path).await?).compat())
    }

    /// The Unix mode, which exists only for entries created on a Unix host.
    fn unix_permissions(stored: &CDR) -> Option<u16> {
        (stored.cdrh.v_made_by >> 8 == UNIX_HOST).then_some((stored.cdrh.exter_attr >> 16) as u16)
    }

    /// The modification time stored in the MS-DOS format, or `None` when it is not a valid date.
    fn modified(stored: &CDR) -> Option<DateTime<Utc>> {
        let (date, time) = (stored.cdrh.mod_date, stored.cdrh.mod_time);
        let result = Utc.with_ymd_and_hms(
            i32::from((date & 0xFE00) >> 9) + 1980,
            u32::from((date & 0x1E0) >> 5),
            u32::from(date & 0x1F),
            u32::from((time & 0xF800) >> 11),
            u32::from((time & 0x7E0) >> 5),
            u32::from((time & 0x1F) << 1),
        );
        match result {
            LocalResult::Single(value) => Some(value),
            _ => None,
        }
    }

    async fn reject_encrypted_entries(path: &Path, entries: &[CDR]) -> Result<()> {
        let mut file = tokio::fs::File::open(path).await?;
        for entry in entries {
            let header_offset = entry.lfh_offset().map_err(Self::map_error)?;
            file.seek(std::io::SeekFrom::Start(header_offset)).await?;
            let mut header = [0_u8; 8];
            file.read_exact(&mut header).await?;
            if &header[..4] != b"PK\x03\x04" {
                return Err(ArchiveError::invalid_format(format!("invalid ZIP local header at offset {header_offset}")));
            }
            let flags = u16::from_le_bytes(header[6..8].try_into().expect("two-byte ZIP flags"));
            if flags & 1 != 0 {
                return Err(Error::new(ErrorKind::PasswordRequired).with_message("ZIP archive is encrypted"));
            }
        }
        Ok(())
    }

    fn decode_name(raw: &[u8], legacy: ZipLegacyEncoding) -> Result<String> {
        if let Ok(value) = std::str::from_utf8(raw) {
            return Ok(value.to_string());
        }
        match legacy {
            ZipLegacyEncoding::None => Err(ArchiveError::invalid_format(format!("ZIP entry name is not UTF-8: {raw:?}"))),
            ZipLegacyEncoding::Cp932 => {
                let (decoded, had_errors) = SHIFT_JIS.decode_without_bom_handling(raw);
                if had_errors {
                    Err(ArchiveError::invalid_format(format!("ZIP entry name is not valid CP932: {raw:?}")))
                } else {
                    Ok(decoded.into_owned())
                }
            }
        }
    }

    fn classify(mode: Option<u16>, name_is_directory: bool, path: &str) -> Result<FsEntryKind> {
        let file_type = mode.unwrap_or(0) & UNIX_FILE_TYPE_MASK;
        match file_type {
            UNIX_SYMLINK => {
                if name_is_directory {
                    Err(ArchiveError::invalid_format(format!("ZIP symlink is also marked as directory: {path}")))
                } else {
                    Ok(FsEntryKind::Link)
                }
            }
            UNIX_DIRECTORY => {
                if name_is_directory {
                    Ok(FsEntryKind::Dir)
                } else {
                    Err(ArchiveError::invalid_format(format!("ZIP directory has no trailing slash: {path}")))
                }
            }
            UNIX_REGULAR_FILE | 0 => Ok(if name_is_directory { FsEntryKind::Dir } else { FsEntryKind::File }),
            _ => Err(Error::new(ErrorKind::UnsupportedType).with_message(format!("unsupported ZIP entry type: {path}"))),
        }
    }

    fn map_error(error: ZipError) -> Error {
        match error {
            error @ ZipError::FeatureNotSupported("encryption") => Error::from_error(error, ErrorKind::PasswordRequired).with_message("ZIP archive is encrypted"),
            error @ (ZipError::FeatureNotSupported(_) | ZipError::CompressionNotSupported(_) | ZipError::AttributeCompatibilityNotSupported(_)) => {
                Error::from_error(error, ErrorKind::UnsupportedType)
            }
            ZipError::UpstreamReadError(error) => Error::from(error),
            error => ArchiveError::invalid_format_from(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_encoding_test() {
        let (bytes, _, _) = SHIFT_JIS.encode("画像/猫.txt");
        assert_eq!(ZipArchive::decode_name(&bytes, ZipLegacyEncoding::Cp932).unwrap(), "画像/猫.txt");
        assert_eq!(ZipArchive::decode_name(&bytes, ZipLegacyEncoding::None).unwrap_err().kind(), &ErrorKind::InvalidFormat);
    }

    #[test]
    fn classifies_symlink_and_rejects_special_type_test() {
        assert_eq!(ZipArchive::classify(Some(UNIX_SYMLINK | 0o777), false, "link").unwrap(), FsEntryKind::Link);
        assert_eq!(ZipArchive::classify(Some(0o010000), false, "fifo").unwrap_err().kind(), &ErrorKind::UnsupportedType);
    }

    #[test]
    fn adapter_keeps_zip_error_as_source_test() {
        let error = ZipArchive::map_error(ZipError::FeatureNotSupported("encryption"));

        assert_eq!(error.kind(), &ErrorKind::PasswordRequired);
        assert!(std::error::Error::source(&error).is_some());
    }
}
