use std::path::Path;

use tokio::process::Command;

use super::{ProcessPriority, process};
use crate::prelude::*;

impl super::MediaTools {
    /// libheif の heif-dec で、HEIF の主画像を PNG にする。タイルの組み立てと、`irot` と `imir` の回転と反転は heif-dec が反映する。
    pub async fn decode_heif(&self, file: &Path, priority: ProcessPriority) -> Result<Vec<u8>> {
        let output_dir = tempfile::tempdir()?;
        let output_path = output_dir.path().join("image.png");

        let mut command = Command::new(&self.options.heif_dec_path);
        command.arg(file).arg(&output_path);
        self.run_with(command, priority).await?;
        // 最上位の画像を複数持つ file は、heif-dec が名前に番号を付けて書き分けるので、この名前の file ができない。
        process::read_output(&output_path, file).await
    }
}
