use tokio_util::bytes::Bytes;

use crate::{
    core::work::WorkClass,
    model::{ImageAnchorPositionType, ImageFormatType, ImageResizeType},
};

/// 何を生成するか。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThumbnailVariant {
    /// 一覧のタイルに出す縮小画像。動画では 1 枚目のフレームから作る。
    Thumbnail,
    /// ビューアが最初に出す、縦横比を保って表示用の段階に収めた画像。
    Display,
    /// 動画のフレームを横 1 列に並べた画像。
    Sprite,
}

/// サムネイルの指定。幅と高さは、サイズ段階へ切り上げる前の値を渡す。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailRequest {
    pub variant: ThumbnailVariant,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub format: ImageFormatType,
    pub resize: ImageResizeType,
    pub anchor: ImageAnchorPositionType,
    /// 生成の枠を待つ段。表示中の画像は Interactive、先読みとサムネイルは Foreground にする。
    pub class: WorkClass,
}

/// 生成した画像。`key` はキャッシュのキーで、応答の `ETag` に使える。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thumbnail {
    pub bytes: Bytes,
    pub format: ImageFormatType,
    /// スプライトシートだけが持つ、並べたフレームの数。
    pub frame_count: Option<u32>,
    pub key: Option<String>,
}
