use std::{fs::File, io::BufReader, path::Path};

use exif::{In, Reader, Tag, Value};
use image::{ImageDecoder as _, ImageReader, metadata::Orientation};
use serde::Deserialize;

use crate::prelude::*;

/// 画像の形式、寸法、撮影情報。位置情報は読まない。
#[derive(Debug, Clone, PartialEq)]
pub struct ImageInfo {
    /// 拡張子から決めた MIME type。
    pub content_type: String,
    /// `image` crate で読む形式は、EXIF の向きを反映した後の幅と高さ。ffmpeg で読む形式は、符号化されたままの幅と高さ。
    pub width: u32,
    pub height: u32,
    /// 保存された画像を、表示のために反時計回りに回す角度（0、90、180、270 度）。`width` と `height` には反映していない。
    /// ffmpeg で読む形式だけが、0 以外になりうる。
    pub rotation: u16,
    /// アルファの補助画像を持つか。ffprobe が gray の映像 stream と色の映像 stream の両方を返したときに true にする。
    pub has_alpha: bool,
    pub exif: ImageExif,
}

/// Exif から読む撮影情報。Exif を持たない、または読めない項目は `None` にする。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ImageExif {
    /// 撮影日時。タイムゾーンを持たない `YYYY-MM-DDTHH:MM:SS` で表す。
    pub taken_at: Option<String>,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub lens_model: Option<String>,
    pub focal_length_mm: Option<f64>,
    pub f_number: Option<f64>,
    pub exposure_time_secs: Option<f64>,
    pub iso: Option<u32>,
}

/// 画像の file から寸法と撮影情報を読む。
///
/// どちらも file を読み続けるので、非同期の呼び出し元は blocking worker 上で呼ぶ。
pub struct ImageInspector;

impl ImageInspector {
    /// `image` crate で読める画像の寸法を、ヘッダだけから得る。向きが縦横を入れ替えるときは入れ替える。
    pub fn read_size(path: &Path) -> Result<(u32, u32)> {
        let mut decoder = ImageReader::open(path)?.with_guessed_format()?.into_decoder()?;
        let (width, height) = decoder.dimensions();
        let swaps_axes = matches!(
            decoder.orientation()?,
            Orientation::Rotate90 | Orientation::Rotate270 | Orientation::Rotate90FlipH | Orientation::Rotate270FlipH
        );
        Ok(if swaps_axes { (height, width) } else { (width, height) })
    }

    /// 撮影情報を読む。Exif を持たない形式や壊れた Exif は、エラーにせず空で返す。
    pub fn read_exif(path: &Path) -> ImageExif {
        let Ok(file) = File::open(path) else {
            return ImageExif::default();
        };
        let Ok(exif) = Reader::new().read_from_container(&mut BufReader::new(file)) else {
            return ImageExif::default();
        };
        let field = |tag| exif.get_field(tag, In::PRIMARY).map(|field| &field.value);
        ImageExif {
            taken_at: field(Tag::DateTimeOriginal).and_then(Self::ascii).and_then(|value| Self::iso_datetime(&value)),
            camera_make: field(Tag::Make).and_then(Self::ascii),
            camera_model: field(Tag::Model).and_then(Self::ascii),
            lens_model: field(Tag::LensModel).and_then(Self::ascii),
            focal_length_mm: field(Tag::FocalLength).and_then(Self::rational),
            f_number: field(Tag::FNumber).and_then(Self::rational),
            exposure_time_secs: field(Tag::ExposureTime).and_then(Self::rational),
            iso: field(Tag::PhotographicSensitivity).and_then(|value| value.get_uint(0)),
        }
    }

    fn ascii(value: &Value) -> Option<String> {
        let Value::Ascii(lines) = value else {
            return None;
        };
        let text = String::from_utf8_lossy(lines.first()?).trim().to_string();
        (!text.is_empty()).then_some(text)
    }

    fn rational(value: &Value) -> Option<f64> {
        let Value::Rational(values) = value else {
            return None;
        };
        let value = values.first()?;
        (value.denom != 0).then(|| value.to_f64()).filter(|value| value.is_finite())
    }

    /// Exif の `YYYY:MM:DD HH:MM:SS` を `YYYY-MM-DDTHH:MM:SS` へ直す。形が違えば `None` にする。
    fn iso_datetime(value: &str) -> Option<String> {
        let (date, time) = value.split_once(' ')?;
        let valid = |part: &str, lengths: &[usize]| {
            let fields: Vec<&str> = part.split(':').collect();
            fields.len() == lengths.len()
                && fields
                    .iter()
                    .zip(lengths)
                    .all(|(field, length)| field.len() == *length && field.bytes().all(|byte| byte.is_ascii_digit()))
        };
        (valid(date, &[4, 2, 2]) && valid(time, &[2, 2, 2])).then(|| format!("{}T{time}", date.replace(':', "-")))
    }
}

/// ffprobe が返す、画像 1 枚分の stream の寸法と回転。ffmpeg で読む形式（HEIC、AVIF など）の寸法に使う。
#[derive(Deserialize)]
struct ProbeImageOutput {
    streams: Vec<ProbeImageStream>,
    /// タイルに分けて符号化された画像の、タイルを束ねる group。`-show_stream_groups` を付けたときだけ出力される。
    #[serde(default)]
    stream_groups: Vec<ProbeStreamGroup>,
}

#[derive(Deserialize)]
struct ProbeImageStream {
    codec_type: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    pix_fmt: Option<String>,
    #[serde(default)]
    side_data_list: Vec<ProbeSideData>,
}

#[derive(Deserialize)]
struct ProbeStreamGroup {
    #[serde(rename = "type")]
    group_type: Option<String>,
    #[serde(default)]
    components: Vec<ProbeTileGrid>,
}

/// Tile Grid の group の component。組み立てた後の画像の幅と高さと、回転を持つ。
#[derive(Deserialize)]
struct ProbeTileGrid {
    width: Option<u32>,
    height: Option<u32>,
    #[serde(default)]
    side_data_list: Vec<ProbeSideData>,
}

#[derive(Deserialize)]
struct ProbeSideData {
    side_data_type: Option<String>,
    rotation: Option<f64>,
}

impl ProbeSideData {
    /// Display Matrix の回転を、反時計回りの 0、90、180、270 度に直す。ないときと、90 の倍数でないときは 0 にする。
    fn display_rotation(side_data_list: &[Self]) -> u16 {
        side_data_list
            .iter()
            .filter(|side_data| side_data.side_data_type.as_deref() == Some("Display Matrix"))
            .find_map(|side_data| side_data.rotation)
            .filter(|rotation| rotation.is_finite() && rotation % 90.0 == 0.0)
            .map_or(0, |rotation| (rotation as i64).rem_euclid(360) as u16)
    }
}

impl ProbeImageOutput {
    /// 最初の Tile Grid の寸法。タイルの stream はタイル 1 枚の寸法しか持たないので、こちらを優先する。
    fn tile_grid_size(&self) -> Option<(u32, u32, u16)> {
        self.stream_groups
            .iter()
            .filter(|group| group.group_type.as_deref() == Some("Tile Grid"))
            .flat_map(|group| &group.components)
            .find_map(|grid| {
                grid.width
                    .zip(grid.height)
                    .map(|(width, height)| (width, height, ProbeSideData::display_rotation(&grid.side_data_list)))
            })
    }

    fn first_video_stream_size(&self) -> Option<(u32, u32, u16)> {
        self.streams.iter().find(|stream| stream.codec_type.as_deref() == Some("video")).and_then(|stream| {
            stream
                .width
                .zip(stream.height)
                .map(|(width, height)| (width, height, ProbeSideData::display_rotation(&stream.side_data_list)))
        })
    }

    /// gray の映像 stream があり、色の stream もあれば、アルファの補助画像を持つとみなす。
    /// モノクロ画像とモノクロのアルファのように gray が 2 本以上並ぶ場合も拾うため、映像 stream が複数あるときも対象にする。
    /// タイル化されたグレースケール画像はアルファなしでも複数の gray stream を持つため、heif-dec に回る（遅いが正しい）。
    fn has_alpha(&self) -> bool {
        let mut video_count = 0;
        let mut gray = false;
        let mut color = false;
        for stream in self.streams.iter().filter(|stream| stream.codec_type.as_deref() == Some("video")) {
            video_count += 1;
            match stream.pix_fmt.as_deref() {
                Some(pix_fmt) if pix_fmt.starts_with("gray") => gray = true,
                Some(_) => color = true,
                None => {}
            }
        }
        gray && (color || video_count > 1)
    }
}

impl ImageInfo {
    /// ffprobe の出力から、画像の幅と高さと、回転（反時計回りの 0、90、180、270 度）と、アルファの有無を得る。幅と高さは入れ替えない。
    /// Tile Grid があればその寸法を、なければ最初の映像 stream の寸法を使う。
    pub fn parse_probe_size(bytes: &[u8]) -> Result<(u32, u32, u16, bool)> {
        let probe: ProbeImageOutput =
            serde_json::from_slice(bytes).map_err(|error| Error::from_error(error, ErrorKind::UnsupportedType).with_message("invalid image information"))?;
        let (width, height, rotation) = probe
            .tile_grid_size()
            .or_else(|| probe.first_video_stream_size())
            .filter(|(width, height, _)| *width > 0 && *height > 0)
            .ok_or_else(|| Error::new(ErrorKind::UnsupportedType).with_message("image size is unavailable"))?;
        Ok((width, height, rotation, probe.has_alpha()))
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write as _;

    use image::{DynamicImage, ImageEncoder as _, codecs::jpeg::JpegEncoder};
    use testresult::TestResult;

    use super::*;

    /// 指定した Exif の項目を持つ JPEG を作る。ASCII の項目は文字列、有理数の項目は (分子, 分母) で渡す。
    fn jpeg_with_exif(width: u32, height: u32, orientation: u16, ascii: &[(u16, &str)], rationals: &[(u16, (u32, u32))], iso: Option<u16>) -> TestResult<Vec<u8>> {
        let mut jpeg = Vec::new();
        JpegEncoder::new(&mut jpeg).write_image(
            DynamicImage::ImageRgb8(image::RgbImage::new(width, height)).as_bytes(),
            width,
            height,
            image::ExtendedColorType::Rgb8,
        )?;

        // リトルエンディアンの TIFF。IFD0 に向き、Make、Model を、Exif IFD に残りを置く。
        struct Entry {
            tag: u16,
            kind: u16,
            count: u32,
            data: Vec<u8>,
        }
        let mut ifd0 = vec![Entry {
            tag: 0x0112,
            kind: 3,
            count: 1,
            data: orientation.to_le_bytes().to_vec(),
        }];
        let mut exif_ifd = Vec::new();
        for (tag, text) in ascii {
            let mut data = text.as_bytes().to_vec();
            data.push(0);
            let entry = Entry {
                tag: *tag,
                kind: 2,
                count: data.len() as u32,
                data,
            };
            if matches!(tag, 0x010f | 0x0110) { ifd0.push(entry) } else { exif_ifd.push(entry) }
        }
        for (tag, (num, denom)) in rationals {
            let mut data = num.to_le_bytes().to_vec();
            data.extend_from_slice(&denom.to_le_bytes());
            exif_ifd.push(Entry {
                tag: *tag,
                kind: 5,
                count: 1,
                data,
            });
        }
        if let Some(iso) = iso {
            exif_ifd.push(Entry {
                tag: 0x8827,
                kind: 3,
                count: 1,
                data: iso.to_le_bytes().to_vec(),
            });
        }
        ifd0.sort_by_key(|entry| entry.tag);
        exif_ifd.sort_by_key(|entry| entry.tag);
        // Exif IFD へのポインタを IFD0 に足す。値は後で埋める。
        ifd0.push(Entry {
            tag: 0x8769,
            kind: 4,
            count: 1,
            data: vec![0; 4],
        });

        let ifd_len = |entries: &[Entry]| 2 + entries.len() * 12 + 4;
        let overflow = |entries: &[Entry]| {
            entries
                .iter()
                .filter(|entry| entry.data.len() > 4)
                .map(|entry| entry.data.len() + entry.data.len() % 2)
                .sum::<usize>()
        };
        let ifd0_start = 8usize;
        let exif_start = ifd0_start + ifd_len(&ifd0) + overflow(&ifd0);
        ifd0.last_mut().ok_or("empty ifd")?.data = (exif_start as u32).to_le_bytes().to_vec();

        let mut tiff = vec![0x49, 0x49, 0x2a, 0x00, 8, 0, 0, 0];
        for (entries, start) in [(&ifd0, ifd0_start), (&exif_ifd, exif_start)] {
            assert_eq!(tiff.len(), start);
            let mut offset = start + ifd_len(entries);
            let mut table = (entries.len() as u16).to_le_bytes().to_vec();
            let mut extra = Vec::new();
            for entry in entries {
                table.extend_from_slice(&entry.tag.to_le_bytes());
                table.extend_from_slice(&entry.kind.to_le_bytes());
                table.extend_from_slice(&entry.count.to_le_bytes());
                if entry.data.len() <= 4 {
                    let mut inline = entry.data.clone();
                    inline.resize(4, 0);
                    table.extend_from_slice(&inline);
                } else {
                    table.extend_from_slice(&(offset as u32).to_le_bytes());
                    extra.extend_from_slice(&entry.data);
                    if entry.data.len() % 2 == 1 {
                        extra.push(0);
                    }
                    offset += entry.data.len() + entry.data.len() % 2;
                }
            }
            table.extend_from_slice(&[0; 4]);
            tiff.extend_from_slice(&table);
            tiff.extend_from_slice(&extra);
        }

        let mut app1 = b"Exif\0\0".to_vec();
        app1.extend_from_slice(&tiff);
        let mut output = jpeg[..2].to_vec();
        output.extend_from_slice(&[0xff, 0xe1]);
        output.extend_from_slice(&((app1.len() + 2) as u16).to_be_bytes());
        output.extend_from_slice(&app1);
        output.extend_from_slice(&jpeg[2..]);
        Ok(output)
    }

    fn write(dir: &tempfile::TempDir, name: &str, bytes: &[u8]) -> TestResult<std::path::PathBuf> {
        let path = dir.path().join(name);
        std::fs::File::create(&path)?.write_all(bytes)?;
        Ok(path)
    }

    #[test]
    fn reads_the_shooting_information_from_exif_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let bytes = jpeg_with_exif(
            40,
            20,
            1,
            &[(0x010f, "ACME"), (0x0110, "Camera One"), (0xa434, "Lens 50mm"), (0x9003, "2024:05:01 12:34:56")],
            &[(0x920a, (500, 10)), (0x829d, (28, 10)), (0x829a, (1, 250))],
            Some(400),
        )?;
        let path = write(&dir, "a.jpg", &bytes)?;

        assert_eq!(ImageInspector::read_size(&path)?, (40, 20));
        assert_eq!(
            ImageInspector::read_exif(&path),
            ImageExif {
                taken_at: Some("2024-05-01T12:34:56".to_string()),
                camera_make: Some("ACME".to_string()),
                camera_model: Some("Camera One".to_string()),
                lens_model: Some("Lens 50mm".to_string()),
                focal_length_mm: Some(50.0),
                f_number: Some(2.8),
                exposure_time_secs: Some(0.004),
                iso: Some(400),
            }
        );
        Ok(())
    }

    #[test]
    fn swaps_the_size_when_the_orientation_turns_the_image_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        for (orientation, expected) in [(1, (40, 20)), (3, (40, 20)), (6, (20, 40)), (8, (20, 40))] {
            let path = write(&dir, "a.jpg", &jpeg_with_exif(40, 20, orientation, &[], &[], None)?)?;
            assert_eq!(ImageInspector::read_size(&path)?, expected, "orientation {orientation}");
        }
        Ok(())
    }

    #[test]
    fn an_image_without_exif_has_empty_shooting_information_test() -> TestResult {
        let dir = tempfile::tempdir()?;
        let png = dir.path().join("a.png");
        image::RgbaImage::new(8, 4).save(&png)?;
        assert_eq!(ImageInspector::read_size(&png)?, (8, 4));
        assert_eq!(ImageInspector::read_exif(&png), ImageExif::default());

        let gif = dir.path().join("a.gif");
        image::RgbaImage::new(8, 4).save(&gif)?;
        assert_eq!(ImageInspector::read_exif(&gif), ImageExif::default(), "kamadak-exif が読まない形式");
        Ok(())
    }

    #[test]
    fn a_malformed_datetime_is_dropped_test() {
        assert_eq!(ImageInspector::iso_datetime("2024:05:01 12:34:56"), Some("2024-05-01T12:34:56".to_string()));
        for value in ["", "2024-05-01 12:34:56", "2024:05:01", "2024:05:01 12:34", "xxxx:05:01 12:34:56"] {
            assert_eq!(ImageInspector::iso_datetime(value), None, "{value}");
        }
    }

    #[test]
    fn parses_the_size_of_the_first_video_stream_test() -> TestResult {
        let bytes = br#"{"streams":[{"codec_type":"audio"},{"codec_type":"video","width":4032,"height":3024}]}"#;
        assert_eq!(ImageInfo::parse_probe_size(bytes)?, (4032, 3024, 0, false));
        assert!(ImageInfo::parse_probe_size(br#"{"streams":[]}"#).is_err());
        assert!(ImageInfo::parse_probe_size(br#"{"streams":[{"codec_type":"video","width":0,"height":0}]}"#).is_err());
        Ok(())
    }

    /// 回転の値は、ffprobe の Display Matrix が返す、符号つきの度数である。
    fn parse_with_side_data(side_data: &str) -> Result<(u32, u32, u16, bool)> {
        let json = format!(r#"{{"streams":[{{"codec_type":"video","width":64,"height":32,"side_data_list":{side_data}}}]}}"#);
        ImageInfo::parse_probe_size(json.as_bytes())
    }

    #[test]
    fn normalizes_the_display_matrix_rotation_without_swapping_the_size_test() -> TestResult {
        for (value, expected) in [(90, 90), (-90, 270), (180, 180), (-180, 180), (270, 270), (-270, 90), (0, 0), (360, 0)] {
            let side_data = format!(r#"[{{"side_data_type":"Display Matrix","rotation":{value}}}]"#);
            assert_eq!(parse_with_side_data(&side_data)?, (64, 32, expected, false), "rotation {value}");
        }
        assert_eq!(parse_with_side_data(r#"[{"side_data_type":"Display Matrix","rotation":90.0}]"#)?, (64, 32, 90, false));
        Ok(())
    }

    #[test]
    fn a_missing_or_unusable_rotation_is_zero_test() -> TestResult {
        for side_data in [
            "[]",
            r#"[{"side_data_type":"Display Matrix"}]"#,
            r#"[{"side_data_type":"Display Matrix","rotation":45}]"#,
            r#"[{"side_data_type":"Display Matrix","rotation":90.5}]"#,
            r#"[{"side_data_type":"Mastering display metadata","rotation":90}]"#,
        ] {
            assert_eq!(parse_with_side_data(side_data)?, (64, 32, 0, false), "{side_data}");
        }
        let without_side_data = br#"{"streams":[{"codec_type":"video","width":64,"height":32}]}"#;
        assert_eq!(ImageInfo::parse_probe_size(without_side_data)?, (64, 32, 0, false));
        Ok(())
    }

    #[test]
    fn a_tile_grid_gives_the_size_of_the_assembled_image_test() -> TestResult {
        // ffprobe 9.0 が、512×512 のタイル 6 枚で 1280×960 を符号化し、`irot` で 90 度回す HEIC に返す形。
        let bytes = br#"{"streams":[{"codec_type":"video","width":512,"height":512},{"codec_type":"video","width":512,"height":512}],
            "stream_groups":[{"type":"Tile Grid","components":[{"nb_tiles":6,"coded_width":1536,"coded_height":1024,"width":1280,"height":960,
            "side_data_list":[{"side_data_type":"Display Matrix","rotation":90}]}]}]}"#;
        assert_eq!(ImageInfo::parse_probe_size(bytes)?, (1280, 960, 90, false));

        let other_group = br#"{"streams":[{"codec_type":"video","width":64,"height":32}],"stream_groups":[{"type":"LCEVC","components":[{"width":128,"height":64}]}]}"#;
        assert_eq!(ImageInfo::parse_probe_size(other_group)?, (64, 32, 0, false));
        let no_groups = br#"{"streams":[{"codec_type":"video","width":64,"height":32}],"stream_groups":[]}"#;
        assert_eq!(ImageInfo::parse_probe_size(no_groups)?, (64, 32, 0, false));
        Ok(())
    }

    #[test]
    fn a_gray_video_stream_next_to_a_color_stream_is_an_alpha_image_test() -> TestResult {
        let untiled = br#"{"streams":[{"codec_type":"video","width":512,"height":512,"pix_fmt":"yuvj420p"},{"codec_type":"video","width":512,"height":512,"pix_fmt":"gray"}]}"#;
        assert_eq!(ImageInfo::parse_probe_size(untiled)?, (512, 512, 0, true));

        let tiled = br#"{"streams":[{"codec_type":"video","width":512,"height":512,"pix_fmt":"yuvj420p"},{"codec_type":"video","width":512,"height":512,"pix_fmt":"gray"}],
            "stream_groups":[{"type":"Tile Grid","components":[{"width":1024,"height":1024}]}]}"#;
        assert_eq!(ImageInfo::parse_probe_size(tiled)?, (1024, 1024, 0, true));

        let grayscale_only = br#"{"streams":[{"codec_type":"video","width":512,"height":512,"pix_fmt":"gray"}]}"#;
        assert_eq!(ImageInfo::parse_probe_size(grayscale_only)?, (512, 512, 0, false));

        let embedded = br#"{"streams":[{"codec_type":"video","width":512,"height":512,"pix_fmt":"rgba"}]}"#;
        assert_eq!(ImageInfo::parse_probe_size(embedded)?, (512, 512, 0, false));

        let gray_and_gray = br#"{"streams":[{"codec_type":"video","width":512,"height":512,"pix_fmt":"gray"},{"codec_type":"video","width":512,"height":512,"pix_fmt":"gray"}]}"#;
        assert_eq!(ImageInfo::parse_probe_size(gray_and_gray)?, (512, 512, 0, true), "モノクロ画像 + モノクロのアルファ");
        Ok(())
    }
}
