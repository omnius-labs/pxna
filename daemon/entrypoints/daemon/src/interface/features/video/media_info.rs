use omnius_pxna_engine::service::{ImageExif, ImageInfo, MediaInfo, VideoInfo};
use omnius_pxna_interface::{models, types::Nullable};

/// engine のメディア情報を、API の応答の型へ写したもの。
pub struct MediaInfoModel(pub models::MediaInfo);

impl From<MediaInfo> for MediaInfoModel {
    fn from(info: MediaInfo) -> Self {
        Self(match info {
            MediaInfo::Video(video) => {
                let mut model = models::MediaInfo::new(models::MediaInfoKind::Video);
                model.video = Some(Self::video(video));
                model
            }
            MediaInfo::Image(image) => {
                let mut model = models::MediaInfo::new(models::MediaInfoKind::Image);
                model.image = Some(Self::image(image));
                model
            }
        })
    }
}

impl MediaInfoModel {
    fn video(info: VideoInfo) -> models::VideoMediaInfo {
        let mut model = models::VideoMediaInfo::new(
            info.container,
            info.video_codec,
            info.duration_secs,
            info.width,
            info.height,
            info.frame_rate,
            info.content_type,
        );
        model.audio_codec = info.audio_codec.map(Nullable::Present);
        model
    }

    /// 回転は、あるときだけ項目として返す。幅と高さには反映しない。
    fn image(info: ImageInfo) -> models::ImageMediaInfo {
        let mut model = models::ImageMediaInfo::new(info.content_type, info.width, info.height, Self::shooting(info.exif));
        model.rotation = (info.rotation != 0).then_some(u32::from(info.rotation));
        model
    }

    fn shooting(exif: ImageExif) -> models::ShootingInfo {
        let mut model = models::ShootingInfo::new();
        model.taken_at = exif.taken_at;
        model.camera_make = exif.camera_make;
        model.camera_model = exif.camera_model;
        model.lens_model = exif.lens_model;
        model.focal_length_mm = exif.focal_length_mm;
        model.f_number = exif.f_number;
        model.exposure_time_secs = exif.exposure_time_secs;
        model.iso = exif.iso;
        model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image_model(rotation: u16) -> Option<models::ImageMediaInfo> {
        let info = ImageInfo {
            content_type: "image/avif".to_string(),
            width: 64,
            height: 32,
            rotation,
            exif: ImageExif::default(),
        };
        MediaInfoModel::from(MediaInfo::Image(info)).0.image
    }

    #[test]
    fn a_rotation_is_returned_only_when_the_image_has_one_without_touching_the_size_test() {
        let summary = |rotation| image_model(rotation).map(|model| (model.width, model.height, model.rotation));
        assert_eq!(summary(0), Some((64, 32, None)));
        assert_eq!(summary(90), Some((64, 32, Some(90))));
        assert_eq!(summary(180), Some((64, 32, Some(180))));
        assert_eq!(summary(270), Some((64, 32, Some(270))));
    }

    #[test]
    fn an_image_without_a_rotation_has_no_rotation_property_in_json_test() -> testresult::TestResult {
        let json = serde_json::to_value(image_model(0).ok_or("an image must give an image model")?)?;
        assert!(json.get("rotation").is_none());
        let json = serde_json::to_value(image_model(90).ok_or("an image must give an image model")?)?;
        assert_eq!(json["rotation"], 90);
        Ok(())
    }
}
