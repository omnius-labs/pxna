use std::{borrow::Cow, io::Cursor};

use tracing::field::Empty;

use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};
use image::{DynamicImage, ImageDecoder as _, ImageReader, Rgba, RgbaImage, codecs::jpeg::JpegEncoder, imageops};

use super::WebpEncoder;
use crate::{
    model::{ImageAnchorPositionType, ImageFormatType, ImageResamplerType, ImageResizeType},
    perf_span,
    prelude::*,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageConvertOptions {
    pub format_type: ImageFormatType,
    /// 非可逆の形式（WebP、JPEG）の圧縮の品質（1 から 100）。PNG では使わない。
    pub quality: u8,
    pub resize_type: ImageResizeType,
    pub resampler_type: ImageResamplerType,
    pub anchor_position_type: ImageAnchorPositionType,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

/// 画像の decode、縮小、encode を受け持つ。
///
/// どの処理も CPU を使い続けるので、非同期の呼び出し元は blocking worker 上で呼ぶ。
pub struct ImageConverter;

impl ImageConverter {
    /// Decodes, resizes, and encodes in one call. The generator decodes once and fans out with `resize_ref`, so only tests use this.
    #[cfg(test)]
    pub fn convert(input: &[u8], opts: &ImageConvertOptions) -> Result<Vec<u8>> {
        Self::validate(opts)?;
        let span = perf_span!("image_convert", input_bytes = input.len(), format = ?opts.format_type, output_bytes = Empty).entered();
        let image = Self::resize(Self::decode(input)?, opts)?;
        let output = Self::encode(&image, opts.format_type, opts.quality)?;
        span.record("output_bytes", output.len());
        Ok(output)
    }

    /// EXIF の向きを反映して読む。反映しないと、縦に撮った写真が横向きのまま並ぶ。
    pub fn decode(input: &[u8]) -> Result<DynamicImage> {
        let span = perf_span!("image_decode", width = Empty, height = Empty).entered();
        let mut decoder = ImageReader::new(Cursor::new(input)).with_guessed_format()?.into_decoder()?;
        let orientation = decoder.orientation()?;
        let mut image = DynamicImage::from_decoder(decoder)?;
        image.apply_orientation(orientation);
        span.record("width", image.width()).record("height", image.height());
        Ok(image)
    }

    pub fn resize(image: DynamicImage, opts: &ImageConvertOptions) -> Result<DynamicImage> {
        Ok(Self::resize_inner(&image, opts)?.unwrap_or(image))
    }

    /// Resizes without consuming the source, so that one decoded image can produce several outputs.
    /// A `Fit` source that already fits is borrowed as it is; every other case is a new image.
    pub fn resize_ref<'a>(image: &'a DynamicImage, opts: &ImageConvertOptions) -> Result<Cow<'a, DynamicImage>> {
        Ok(match Self::resize_inner(image, opts)? {
            Some(resized) => Cow::Owned(resized),
            None => Cow::Borrowed(image),
        })
    }

    /// Returns `None` when the source is already the result, which only happens to a `Fit` source that fits the box.
    fn resize_inner(image: &DynamicImage, opts: &ImageConvertOptions) -> Result<Option<DynamicImage>> {
        Self::validate(opts)?;
        let span = perf_span!("image_resize", source_width = image.width(), source_height = image.height(), width = Empty, height = Empty).entered();

        let (src_w, src_h) = (image.width() as f32, image.height() as f32);
        let (target_w, target_h) = match (opts.width, opts.height) {
            (Some(w), Some(h)) => (w, h),
            (Some(w), None) => (w, ((w as f32) * (src_h / src_w)).round().max(1.0) as u32),
            (None, Some(h)) => (((h as f32) * (src_w / src_h)).round().max(1.0) as u32, h),
            (None, None) => unreachable!("validate rejects a request without a width and a height"),
        };
        let filter = match opts.resampler_type {
            ImageResamplerType::Lanczos3 => FilterType::Lanczos3,
            ImageResamplerType::Unknown => unreachable!("validate rejects an unknown resampler"),
        };

        let resized = match opts.resize_type {
            // 縦横比を無視して、枠の大きさにする。
            ImageResizeType::Manual => Self::scale(image, target_w, target_h, filter)?,
            // 枠を覆うまで拡大縮小し、anchor の位置で枠の大きさに切り抜く。
            ImageResizeType::Crop => {
                let scale = f32::max(target_w as f32 / src_w, target_h as f32 / src_h);
                let scaled = Self::scale(image, (src_w * scale).ceil().max(1.0) as u32, (src_h * scale).ceil().max(1.0) as u32, filter)?;
                let (x, y) = Self::anchor_offset(opts.anchor_position_type, scaled.width(), scaled.height(), target_w, target_h);
                scaled.crop_imm(x, y, target_w, target_h)
            }
            // 枠に収まるまで拡大縮小し、透明な余白で枠を埋める。
            ImageResizeType::Pad => {
                let scale = f32::min(target_w as f32 / src_w, target_h as f32 / src_h);
                let scaled = Self::scale(image, (src_w * scale).floor().max(1.0) as u32, (src_h * scale).floor().max(1.0) as u32, filter)?;
                let (x, y) = Self::anchor_offset(opts.anchor_position_type, target_w, target_h, scaled.width(), scaled.height());
                let mut canvas = RgbaImage::from_pixel(target_w, target_h, Rgba([0; 4]));
                imageops::replace(&mut canvas, &scaled.to_rgba8(), x.into(), y.into());
                DynamicImage::ImageRgba8(canvas)
            }
            // 縦横比を保って枠に収める。元の画像より大きくはせず、余白も付けない。
            ImageResizeType::Fit => {
                if image.width() <= target_w && image.height() <= target_h {
                    span.record("width", image.width()).record("height", image.height());
                    return Ok(None);
                }
                let ratio = f64::min(target_w as f64 / image.width() as f64, target_h as f64 / image.height() as f64);
                let width = ((image.width() as f64 * ratio).round() as u32).clamp(1, target_w);
                let height = ((image.height() as f64 * ratio).round() as u32).clamp(1, target_h);
                Self::scale(image, width, height, filter)?
            }
            ImageResizeType::Unknown => unreachable!("validate rejects an unknown resize type"),
        };
        span.record("width", resized.width()).record("height", resized.height());
        Ok(Some(resized))
    }

    /// SIMD で width × height へ縮小する。`image` crate の `resize_exact` は、同じ Lanczos3 でも数倍遅い。
    fn scale(image: &DynamicImage, width: u32, height: u32, filter: FilterType) -> Result<DynamicImage> {
        let mut output = DynamicImage::new(width, height, image.color());
        let options = ResizeOptions::new().resize_alg(ResizeAlg::Convolution(filter));
        Resizer::new()
            .resize(image, &mut output, &options)
            .map_err(|error| Error::from_error(error, ErrorKind::ImageError))?;
        Ok(output)
    }

    pub fn encode(image: &DynamicImage, format_type: ImageFormatType, quality: u8) -> Result<Vec<u8>> {
        let span = perf_span!("image_encode", format = ?format_type, bytes = Empty).entered();
        let mut output = Vec::new();
        match format_type {
            ImageFormatType::Png => image.write_to(&mut Cursor::new(&mut output), image::ImageFormat::Png)?,
            ImageFormatType::Jpeg => {
                let rgb = Self::flatten_on_white(image);
                JpegEncoder::new_with_quality(&mut output, quality).encode_image(&rgb)?;
            }
            ImageFormatType::Webp => output = WebpEncoder::encode_lossy(image, quality)?,
            ImageFormatType::Unknown => return Err(Error::new(ErrorKind::InvalidInput).with_message("unknown image format")),
        }
        span.record("bytes", output.len());
        Ok(output)
    }

    /// 変換の指定が正しいかを確かめる。変換に入る前に、キャッシュを引くより先に呼べる。
    pub fn validate(opts: &ImageConvertOptions) -> Result<()> {
        let invalid = |message: &str| Err(Error::new(ErrorKind::InvalidInput).with_message(message.to_string()));

        if opts.width.is_none() && opts.height.is_none() {
            return invalid("width or height is required");
        }
        if opts.width == Some(0) || opts.height == Some(0) {
            return invalid("width and height must be greater than zero");
        }
        if opts.format_type == ImageFormatType::Unknown {
            return invalid("unknown image format");
        }
        if !(1..=100).contains(&opts.quality) {
            return invalid("quality must be between 1 and 100");
        }
        if opts.resize_type == ImageResizeType::Unknown {
            return invalid("unknown resize type");
        }
        if opts.resampler_type == ImageResamplerType::Unknown {
            return invalid("unknown resampler");
        }
        // JPEG は透明を表せないので、透明な余白を作る pad とは組めない。
        if opts.resize_type == ImageResizeType::Pad && opts.format_type == ImageFormatType::Jpeg {
            return invalid("pad cannot be combined with jpeg because jpeg has no transparency");
        }
        Ok(())
    }

    /// 画像を左から順に、横 1 列に並べる。高さは最も高い画像に合わせ、余った部分は透明にする。
    pub fn tile_horizontally(images: &[DynamicImage]) -> DynamicImage {
        let width = images.iter().map(DynamicImage::width).sum();
        let height = images.iter().map(DynamicImage::height).max().unwrap_or_default();

        let mut canvas = RgbaImage::from_pixel(width, height, Rgba([0; 4]));
        let mut x = 0;
        for image in images {
            imageops::replace(&mut canvas, &image.to_rgba8(), i64::from(x), 0);
            x += image.width();
        }
        DynamicImage::ImageRgba8(canvas)
    }

    /// 透明な部分を白の背景に合成して、alpha のない画像にする。
    fn flatten_on_white(image: &DynamicImage) -> DynamicImage {
        if !image.color().has_alpha() {
            return DynamicImage::ImageRgb8(image.to_rgb8());
        }

        let mut canvas = RgbaImage::from_pixel(image.width(), image.height(), Rgba([255, 255, 255, 255]));
        imageops::overlay(&mut canvas, &image.to_rgba8(), 0, 0);
        DynamicImage::ImageRgb8(DynamicImage::ImageRgba8(canvas).to_rgb8())
    }

    /// 大きい側の `container` の中に、小さい側の `inner` を置くときの左上の位置を、anchor から求める。
    fn anchor_offset(anchor: ImageAnchorPositionType, container_w: u32, container_h: u32, inner_w: u32, inner_h: u32) -> (u32, u32) {
        let dx = container_w.saturating_sub(inner_w);
        let dy = container_h.saturating_sub(inner_h);
        let x = match anchor {
            ImageAnchorPositionType::Left | ImageAnchorPositionType::TopLeft | ImageAnchorPositionType::BottomLeft => 0,
            ImageAnchorPositionType::Right | ImageAnchorPositionType::TopRight | ImageAnchorPositionType::BottomRight => dx,
            _ => dx / 2,
        };
        let y = match anchor {
            ImageAnchorPositionType::Top | ImageAnchorPositionType::TopLeft | ImageAnchorPositionType::TopRight => 0,
            ImageAnchorPositionType::Bottom | ImageAnchorPositionType::BottomLeft | ImageAnchorPositionType::BottomRight => dy,
            _ => dy / 2,
        };
        (x, y)
    }
}

#[cfg(test)]
mod tests {
    use image::ImageFormat;
    use testresult::TestResult;

    use super::*;

    fn options(resize_type: ImageResizeType, format_type: ImageFormatType, width: Option<u32>, height: Option<u32>) -> ImageConvertOptions {
        ImageConvertOptions {
            format_type,
            quality: 80,
            resize_type,
            resampler_type: ImageResamplerType::Lanczos3,
            anchor_position_type: ImageAnchorPositionType::Center,
            width,
            height,
        }
    }

    fn png(width: u32, height: u32, pixel: Rgba<u8>) -> TestResult<Vec<u8>> {
        let mut output = Vec::new();
        RgbaImage::from_pixel(width, height, pixel).write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
        Ok(output)
    }

    fn size(bytes: &[u8]) -> TestResult<(u32, u32)> {
        let image = ImageConverter::decode(bytes)?;
        Ok((image.width(), image.height()))
    }

    #[test]
    fn fit_keeps_the_aspect_ratio_and_never_enlarges_test() -> TestResult {
        let wide = png(400, 200, Rgba([10, 20, 30, 255]))?;
        let shrunk = ImageConverter::convert(&wide, &options(ImageResizeType::Fit, ImageFormatType::Png, Some(100), Some(100)))?;
        assert_eq!(size(&shrunk)?, (100, 50));

        let small = png(40, 20, Rgba([10, 20, 30, 255]))?;
        let kept = ImageConverter::convert(&small, &options(ImageResizeType::Fit, ImageFormatType::Png, Some(1024), Some(1024)))?;
        assert_eq!(size(&kept)?, (40, 20), "元の画像より大きくしない");
        Ok(())
    }

    #[test]
    fn resize_ref_matches_resize_and_keeps_the_source_test() -> TestResult {
        let source = ImageConverter::decode(&png(400, 200, Rgba([10, 20, 30, 255]))?)?;
        for resize_type in [ImageResizeType::Crop, ImageResizeType::Fit, ImageResizeType::Pad, ImageResizeType::Manual] {
            let opts = options(resize_type, ImageFormatType::Png, Some(100), Some(100));
            let expected = ImageConverter::resize(source.clone(), &opts)?;
            // The same decoded image can be resized again and again.
            for _ in 0..2 {
                let resized = ImageConverter::resize_ref(&source, &opts)?;
                assert!(matches!(resized, Cow::Owned(_)), "{resize_type} makes a new image");
                assert_eq!(resized.as_bytes(), expected.as_bytes(), "{resize_type}");
                assert_eq!((resized.width(), resized.height()), (expected.width(), expected.height()), "{resize_type}");
            }
        }
        assert_eq!((source.width(), source.height()), (400, 200));

        let fits = options(ImageResizeType::Fit, ImageFormatType::Png, Some(1024), Some(1024));
        let kept = ImageConverter::resize_ref(&source, &fits)?;
        assert!(matches!(kept, Cow::Borrowed(_)), "a Fit source that already fits is not copied");
        assert_eq!(ImageConverter::resize(source.clone(), &fits)?.as_bytes(), source.as_bytes());
        Ok(())
    }

    #[test]
    fn crop_pad_and_manual_fill_the_box_test() -> TestResult {
        let wide = png(400, 200, Rgba([10, 20, 30, 255]))?;
        for resize_type in [ImageResizeType::Crop, ImageResizeType::Pad, ImageResizeType::Manual] {
            let converted = ImageConverter::convert(&wide, &options(resize_type, ImageFormatType::Png, Some(100), Some(100)))?;
            assert_eq!(size(&converted)?, (100, 100), "{resize_type}");
        }

        let padded = ImageConverter::decode(&ImageConverter::convert(&wide, &options(ImageResizeType::Pad, ImageFormatType::Png, Some(100), Some(100)))?)?.to_rgba8();
        assert_eq!(padded.get_pixel(50, 0)[3], 0, "上の余白は透明");
        assert_eq!(padded.get_pixel(50, 50)[3], 255, "中央の画像は不透明");
        Ok(())
    }

    #[test]
    fn a_single_dimension_keeps_the_aspect_ratio_test() -> TestResult {
        let wide = png(400, 200, Rgba([10, 20, 30, 255]))?;
        let converted = ImageConverter::convert(&wide, &options(ImageResizeType::Manual, ImageFormatType::Png, Some(100), None))?;
        assert_eq!(size(&converted)?, (100, 50));
        Ok(())
    }

    #[test]
    fn encodes_the_requested_format_test() -> TestResult {
        let source = png(40, 20, Rgba([10, 20, 30, 255]))?;
        let jpeg = ImageConverter::convert(&source, &options(ImageResizeType::Fit, ImageFormatType::Jpeg, Some(40), Some(40)))?;
        assert_eq!(image::guess_format(&jpeg)?, ImageFormat::Jpeg);
        let png = ImageConverter::convert(&source, &options(ImageResizeType::Fit, ImageFormatType::Png, Some(40), Some(40)))?;
        assert_eq!(image::guess_format(&png)?, ImageFormat::Png);
        Ok(())
    }

    #[test]
    fn encodes_webp_and_keeps_the_transparent_padding_test() -> TestResult {
        let wide = png(400, 200, Rgba([10, 20, 30, 255]))?;
        let webp = ImageConverter::convert(&wide, &options(ImageResizeType::Pad, ImageFormatType::Webp, Some(100), Some(100)))?;
        assert_eq!(image::guess_format(&webp)?, ImageFormat::WebP);
        let padded = ImageConverter::decode(&webp)?.to_rgba8();
        assert_eq!((padded.width(), padded.height()), (100, 100));
        assert_eq!(padded.get_pixel(50, 0)[3], 0, "上の余白は透明");
        assert_eq!(padded.get_pixel(50, 50)[3], 255, "中央の画像は不透明");
        Ok(())
    }

    #[test]
    fn quality_changes_the_size_of_lossy_formats_only_test() -> TestResult {
        let noisy = {
            let mut output = Vec::new();
            RgbaImage::from_fn(128, 128, |x, y| {
                let v = x.wrapping_mul(2654435761).wrapping_add(y.wrapping_mul(40503)) >> 8;
                Rgba([v as u8, (v >> 3) as u8, (v >> 5) as u8, 255])
            })
            .write_to(&mut Cursor::new(&mut output), ImageFormat::Png)?;
            output
        };
        let convert = |format_type, quality| {
            let opts = ImageConvertOptions {
                quality,
                ..options(ImageResizeType::Fit, format_type, Some(128), Some(128))
            };
            ImageConverter::convert(&noisy, &opts)
        };
        for format_type in [ImageFormatType::Webp, ImageFormatType::Jpeg] {
            assert!(convert(format_type, 30)?.len() < convert(format_type, 90)?.len(), "{format_type}");
        }
        assert_eq!(convert(ImageFormatType::Png, 30)?, convert(ImageFormatType::Png, 90)?, "PNG は品質を使わない");
        Ok(())
    }

    #[test]
    fn flattens_transparency_on_white_for_jpeg_test() -> TestResult {
        let transparent = png(20, 20, Rgba([0, 0, 0, 0]))?;
        let jpeg = ImageConverter::convert(&transparent, &options(ImageResizeType::Fit, ImageFormatType::Jpeg, Some(20), Some(20)))?;
        let pixel = ImageConverter::decode(&jpeg)?.to_rgb8().get_pixel(10, 10).0;
        assert!(pixel.iter().all(|channel| *channel > 250), "透明は白になる: {pixel:?}");
        Ok(())
    }

    #[test]
    fn rejects_invalid_options_test() -> TestResult {
        let source = png(40, 20, Rgba([10, 20, 30, 255]))?;
        let cases = [
            options(ImageResizeType::Pad, ImageFormatType::Jpeg, Some(10), Some(10)),
            options(ImageResizeType::Fit, ImageFormatType::Png, None, None),
            options(ImageResizeType::Fit, ImageFormatType::Png, Some(0), Some(10)),
            options(ImageResizeType::Unknown, ImageFormatType::Png, Some(10), Some(10)),
            options(ImageResizeType::Fit, ImageFormatType::Unknown, Some(10), Some(10)),
            ImageConvertOptions {
                quality: 0,
                ..options(ImageResizeType::Fit, ImageFormatType::Webp, Some(10), Some(10))
            },
            ImageConvertOptions {
                quality: 101,
                ..options(ImageResizeType::Fit, ImageFormatType::Jpeg, Some(10), Some(10))
            },
        ];
        for opts in cases {
            assert_eq!(*ImageConverter::convert(&source, &opts).unwrap_err().kind(), ErrorKind::InvalidInput, "{opts:?}");
        }
        Ok(())
    }

    /// EXIF の向きだけを持つ JPEG を作る。向き 6 は、右へ 90 度回して表示する。
    fn jpeg_with_orientation(width: u32, height: u32, orientation: u8) -> TestResult<Vec<u8>> {
        let mut jpeg = Vec::new();
        JpegEncoder::new(&mut jpeg).encode_image(&DynamicImage::ImageRgb8(image::RgbImage::new(width, height)))?;

        let mut exif = b"Exif\0\0".to_vec();
        exif.extend_from_slice(&[0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00]);
        exif.extend_from_slice(&[0x01, 0x00, 0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, orientation, 0x00, 0x00, 0x00]);
        exif.extend_from_slice(&[0x00, 0x00, 0x00, 0x00]);

        let mut output = jpeg[..2].to_vec();
        output.extend_from_slice(&[0xff, 0xe1]);
        output.extend_from_slice(&((exif.len() + 2) as u16).to_be_bytes());
        output.extend_from_slice(&exif);
        output.extend_from_slice(&jpeg[2..]);
        Ok(output)
    }

    #[test]
    fn applies_the_exif_orientation_test() -> TestResult {
        assert_eq!(size(&jpeg_with_orientation(40, 20, 1)?)?, (40, 20));
        assert_eq!(size(&jpeg_with_orientation(40, 20, 6)?)?, (20, 40), "向き 6 は縦横が入れ替わる");
        Ok(())
    }
}
