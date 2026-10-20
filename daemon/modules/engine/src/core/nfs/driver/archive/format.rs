use std::path::Path;

use crate::prelude::*;

use super::ArchiveError;

const ZIP_LOCAL_FILE: &[u8; 4] = b"PK\x03\x04";
const ZIP_EMPTY: &[u8; 4] = b"PK\x05\x06";
const ZIP_SPANNED: &[u8; 4] = b"PK\x07\x08";
const SEVEN_ZIP: &[u8; 6] = b"7z\xBC\xAF\x27\x1C";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchiveFormat {
    Zip,
    SevenZip,
}

impl ArchiveFormat {
    pub(super) fn detect(head: &[u8]) -> Result<Self> {
        if head.starts_with(ZIP_LOCAL_FILE) || head.starts_with(ZIP_EMPTY) || head.starts_with(ZIP_SPANNED) {
            return Ok(Self::Zip);
        }
        if head.starts_with(SEVEN_ZIP) {
            return Ok(Self::SevenZip);
        }
        Err(Error::new(ErrorKind::UnsupportedType).with_message("unsupported archive signature"))
    }

    pub(crate) fn supports_path(path: &str) -> bool {
        Path::new(path)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("zip") || extension.eq_ignore_ascii_case("7z"))
    }

    pub(super) fn validate_extension(self, path: &Path) -> Result<()> {
        let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
            return Ok(());
        };
        if !Self::supports_path(path.to_str().unwrap_or_default()) {
            return Ok(());
        }
        let matches = match self {
            Self::Zip => extension.eq_ignore_ascii_case("zip"),
            Self::SevenZip => extension.eq_ignore_ascii_case("7z"),
        };
        if matches {
            Ok(())
        } else {
            Err(ArchiveError::invalid_format(format!("archive extension does not match signature: {}", path.display())))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_supported_signatures_test() {
        assert_eq!(ArchiveFormat::detect(b"PK\x03\x04data").unwrap(), ArchiveFormat::Zip);
        assert_eq!(ArchiveFormat::detect(b"PK\x05\x06data").unwrap(), ArchiveFormat::Zip);
        assert_eq!(ArchiveFormat::detect(b"PK\x07\x08data").unwrap(), ArchiveFormat::Zip);
        assert_eq!(ArchiveFormat::detect(b"7z\xBC\xAF\x27\x1Cdata").unwrap(), ArchiveFormat::SevenZip);
    }

    #[test]
    fn rejects_unknown_signature_test() {
        assert_eq!(ArchiveFormat::detect(b"not an archive").unwrap_err().kind(), &ErrorKind::UnsupportedType);
    }

    #[test]
    fn rejects_known_extension_mismatch_test() {
        assert_eq!(
            ArchiveFormat::SevenZip.validate_extension(Path::new("a.zip")).unwrap_err().kind(),
            &ErrorKind::InvalidFormat
        );
        assert_eq!(ArchiveFormat::Zip.validate_extension(Path::new("a.7z")).unwrap_err().kind(), &ErrorKind::InvalidFormat);
        ArchiveFormat::Zip.validate_extension(Path::new("a.bin")).unwrap();
        ArchiveFormat::SevenZip.validate_extension(Path::new("a")).unwrap();
    }

    #[test]
    fn recognizes_supported_extensions_test() {
        assert!(ArchiveFormat::supports_path("/data/a.zip"));
        assert!(ArchiveFormat::supports_path("/data/a.ZIP"));
        assert!(ArchiveFormat::supports_path("a.7z"));
        assert!(!ArchiveFormat::supports_path("/data/a.png"));
        assert!(!ArchiveFormat::supports_path("/data/zip"));
        assert!(!ArchiveFormat::supports_path("/data/a.zip.png"));
    }
}
