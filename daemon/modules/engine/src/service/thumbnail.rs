// thumbnail の生成は engine の公開 API に現れるため、service 層から名前を出す。
pub use crate::core::{
    image::SizeSteps,
    media::{MediaKind, MediaOptions},
    thumbnail::{Thumbnail, ThumbnailGenerator, ThumbnailOptions, ThumbnailRequest, ThumbnailVariant},
};
