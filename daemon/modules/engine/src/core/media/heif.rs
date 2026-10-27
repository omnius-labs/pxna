use std::path::Path;

use tokio::process::Command;

use super::process;
use crate::prelude::*;

impl super::MediaTools {
    /// Decode the primary image of a HEIF file with libheif's heif-dec.
    /// heif-dec assembles tiles and applies `irot` and `imir`.
    ///
    /// It asks for an uncompressed TIFF first: it holds the same 8-bit pixels as a level-0 PNG (with alpha when the
    /// image has it) but skips the PNG filtering pass. libheif 1.19 added TIFF output, but a build may omit libtiff, so
    /// an unsupported output falls back to a level-0 PNG. The PNG fallback needs libheif 1.17 or newer, which the daemon
    /// checks at startup.
    pub async fn decode_heif(&self, file: &Path) -> Result<Vec<u8>> {
        let output_dir = tempfile::tempdir()?;
        let tiff_path = output_dir.path().join("image.tiff");

        let mut command = Command::new(&self.options.heif_dec_path);
        command.arg(file).arg(&tiff_path);
        match self.run(command).await {
            Ok(_) => return process::read_output(&tiff_path, file).await,
            Err(error) if Self::tiff_output_is_unsupported(&error) => {}
            Err(error) => return Err(error),
        }

        let png_path = output_dir.path().join("image.png");
        let mut command = Command::new(&self.options.heif_dec_path);
        command.args(["--png-compression-level", "0"]).arg(file).arg(&png_path);
        self.run(command).await?;
        // A file with several top-level images gets numbered names from heif-dec, so this exact name is not created.
        process::read_output(&png_path, file).await
    }

    /// Whether heif-dec rejected the `.tiff` output itself rather than the image. libheif before 1.19 knows no TIFF
    /// suffix, and a build without libtiff reports that support was not compiled in.
    fn tiff_output_is_unsupported(error: &Error) -> bool {
        error
            .message()
            .is_some_and(|message| message.contains("TIFF support has not been compiled in") || message.contains("Unknown file type"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::MediaTools;
    use crate::prelude::*;

    #[test]
    fn recognizes_a_rejected_tiff_output_test() {
        let unsupported = |message: &str| MediaTools::tiff_output_is_unsupported(&Error::new(ErrorKind::UnsupportedType).with_message(message.to_string()));
        assert!(unsupported("heif-dec failed: TIFF support has not been compiled in."));
        assert!(unsupported("heif-dec failed: Unknown file type in /tmp/image.tiff"));
        assert!(!unsupported("heif-dec failed: Could not decode image: invalid"));
    }
}
