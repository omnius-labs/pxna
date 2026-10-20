/// 拡張子から決める、ファイルの読み方と MIME type。
///
/// archive の判定を拡張子だけで行うのと同じ理由で、archive の内側を一覧するたびに各 entry を開かずに済ませる。
/// 対応する拡張子の集合はサーバの性質であり、クライアントに同じ集合を持たせない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    /// `image` crate で読める画像。
    Image,
    /// `image` crate では読まず、ffmpeg で 1 枚目を取り出して読む画像。
    FfmpegImage,
    /// ffmpeg でフレームを取り出す動画。
    Video,
}

impl MediaKind {
    /// 拡張子と、その読み方と MIME type の表。サムネイルの生成の判定と、一覧に載せる `contentType` が同じ表から引く。
    const TABLE: &[(&str, MediaKind, &str)] = &[
        ("png", Self::Image, "image/png"),
        ("jpg", Self::Image, "image/jpeg"),
        ("jpeg", Self::Image, "image/jpeg"),
        ("jpe", Self::Image, "image/jpeg"),
        ("gif", Self::Image, "image/gif"),
        ("webp", Self::Image, "image/webp"),
        ("bmp", Self::Image, "image/bmp"),
        ("tif", Self::Image, "image/tiff"),
        ("tiff", Self::Image, "image/tiff"),
        ("ico", Self::Image, "image/x-icon"),
        ("tga", Self::Image, "image/x-tga"),
        ("qoi", Self::Image, "image/x-qoi"),
        ("pnm", Self::Image, "image/x-portable-anymap"),
        ("pbm", Self::Image, "image/x-portable-bitmap"),
        ("pgm", Self::Image, "image/x-portable-graymap"),
        ("ppm", Self::Image, "image/x-portable-pixmap"),
        ("hdr", Self::Image, "image/vnd.radiance"),
        ("exr", Self::Image, "image/x-exr"),
        ("heic", Self::FfmpegImage, "image/heic"),
        ("heif", Self::FfmpegImage, "image/heif"),
        ("avif", Self::FfmpegImage, "image/avif"),
        ("jxl", Self::FfmpegImage, "image/jxl"),
        ("jp2", Self::FfmpegImage, "image/jp2"),
        ("mp4", Self::Video, "video/mp4"),
        ("m4v", Self::Video, "video/x-m4v"),
        ("mov", Self::Video, "video/quicktime"),
        ("mkv", Self::Video, "video/x-matroska"),
        ("webm", Self::Video, "video/webm"),
        ("avi", Self::Video, "video/x-msvideo"),
        ("wmv", Self::Video, "video/x-ms-wmv"),
        ("flv", Self::Video, "video/x-flv"),
        ("mpg", Self::Video, "video/mpeg"),
        ("mpeg", Self::Video, "video/mpeg"),
        ("ts", Self::Video, "video/mp2t"),
        ("m2ts", Self::Video, "video/mp2t"),
        ("3gp", Self::Video, "video/3gpp"),
        ("ogv", Self::Video, "video/ogg"),
    ];

    /// ファイル名の拡張子を、大文字と小文字を区別せずに見る。どれにも当たらなければ `None` にする。
    pub fn of_name(name: &str) -> Option<Self> {
        Self::lookup(name).map(|(kind, _)| kind)
    }

    /// 拡張子から決めた MIME type。どれにも当たらなければ `None` にする。
    pub fn content_type(name: &str) -> Option<&'static str> {
        Self::lookup(name).map(|(_, content_type)| content_type)
    }

    fn lookup(name: &str) -> Option<(Self, &'static str)> {
        let (_, extension) = name.rsplit_once('.')?;
        let extension = extension.to_ascii_lowercase();
        Self::TABLE
            .iter()
            .find(|(known, _, _)| *known == extension)
            .map(|(_, kind, content_type)| (*kind, *content_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_by_extension_ignoring_case_test() {
        assert_eq!(MediaKind::of_name("a.PNG"), Some(MediaKind::Image));
        assert_eq!(MediaKind::of_name("photo.final.jpeg"), Some(MediaKind::Image));
        assert_eq!(MediaKind::of_name("IMG_0001.HEIC"), Some(MediaKind::FfmpegImage));
        assert_eq!(MediaKind::of_name("clip.Mp4"), Some(MediaKind::Video));
    }

    #[test]
    fn returns_content_type_by_extension_test() {
        assert_eq!(MediaKind::content_type("a.png"), Some("image/png"));
        assert_eq!(MediaKind::content_type("a.JPG"), Some("image/jpeg"));
        assert_eq!(MediaKind::content_type("IMG_0001.HEIC"), Some("image/heic"));
        assert_eq!(MediaKind::content_type("clip.mp4"), Some("video/mp4"));
        assert_eq!(MediaKind::content_type("notes.txt"), None);
        assert_eq!(MediaKind::content_type("noext"), None);
    }

    #[test]
    fn content_type_prefix_matches_kind_test() {
        for (extension, kind, content_type) in MediaKind::TABLE {
            let name = format!("a.{extension}");
            assert_eq!(MediaKind::of_name(&name), Some(*kind), "{name}");
            let expected = if *kind == MediaKind::Video { "video/" } else { "image/" };
            assert!(content_type.starts_with(expected), "{name}: {content_type}");
        }
    }

    #[test]
    fn rejects_names_without_a_known_extension_test() {
        for name in ["notes.txt", "archive.zip", "png", "", "noext."] {
            assert_eq!(MediaKind::of_name(name), None, "{name}");
        }
    }
}
