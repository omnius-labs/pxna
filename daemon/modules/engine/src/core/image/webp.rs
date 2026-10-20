use image::DynamicImage;
use libwebp_sys::{
    WebPConfig, WebPEncode, WebPEncodingError, WebPMemoryWrite, WebPMemoryWriter, WebPMemoryWriterClear, WebPMemoryWriterInit, WebPPicture, WebPPictureFree, WebPPictureImportRGB,
    WebPPictureImportRGBA, WebPPreset, WebPValidateConfig,
};

use crate::prelude::*;

/// libwebp による非可逆の WebP のエンコード。
///
/// image crate の WebP エンコーダは可逆圧縮だけを持つため、非可逆はここで libwebp を呼ぶ。
/// C の関数を呼ぶ unsafe は `encode_lossy` の中に限る。
pub struct WebpEncoder;

impl WebpEncoder {
    /// WebP の幅と高さの上限。これを超える画像は libwebp が符号化できない。
    const MAX_DIMENSION: u32 = 16383;

    /// libwebp の圧縮の手間（0 から 6）。小さいほど速く、同じ品質でも大きいファイルになりやすい。libwebp の既定は 4。
    ///
    /// 4096 × 2731 の表示用画像を品質 90 で符号化すると、2 は 4 の 4 割から 6 割の時間で済み、PSNR は 0.2 dB 以内、大きさは ±4% に収まった
    /// （[計測記録](../../../../../../docs/measurements/2026-10-16-display-image-latency.md)）。1 以下は大きさが増え、3 以上は 2 より遅い割に小さくならない。
    const METHOD: i32 = 2;

    /// 画像を、品質 `quality`（1 から 100）の非可逆の WebP へ符号化する。透明は保つ。
    pub fn encode_lossy(image: &DynamicImage, quality: u8) -> Result<Vec<u8>> {
        let (width, height) = (image.width(), image.height());
        if width == 0 || height == 0 || width > Self::MAX_DIMENSION || height > Self::MAX_DIMENSION {
            return Err(Error::new(ErrorKind::InvalidInput).with_message(format!("webp cannot encode a {width}x{height} image")));
        }

        // 透明を持たない画像は、不透明の平面を作らず RGB のまま渡す。
        let has_alpha = image.color().has_alpha();
        let (pixels, bytes_per_pixel) = if has_alpha { (image.to_rgba8().into_raw(), 4) } else { (image.to_rgb8().into_raw(), 3) };
        let stride = width as usize * bytes_per_pixel;
        debug_assert_eq!(pixels.len(), stride * height as usize);

        let failed = |message: &str| Error::new(ErrorKind::ImageError).with_message(format!("webp encoding failed: {message}"));
        let mut config = WebPConfig::new_with_preset(WebPPreset::WEBP_PRESET_DEFAULT, f32::from(quality)).map_err(|()| failed("config"))?;
        config.method = Self::METHOD;
        // SAFETY: `config` は初期化済みで、検証は読むだけである。
        if unsafe { WebPValidateConfig(&config) } == 0 {
            return Err(failed("invalid settings"));
        }
        let mut picture = WebPPicture::new().map_err(|()| failed("picture"))?;
        picture.width = width as i32;
        picture.height = height as i32;

        let mut writer = std::mem::MaybeUninit::<WebPMemoryWriter>::uninit();
        // SAFETY: `pixels` は幅 × 高さ × `bytes_per_pixel` byte の連続した領域で、`stride` はその 1 行の byte 数である。
        // 幅と高さは `MAX_DIMENSION` 以下なので `c_int` に収まる。取り込みは `picture` の内部へ画素を複製するため、`pixels` はこの呼び出しの後で使わない。
        // `writer` は `WebPMemoryWriterInit` で初期化してから `picture` に結び付け、`WebPEncode` が終わるまで動かさない。
        // 確保した領域は、成功しても失敗しても、`WebPMemoryWriterClear` と `WebPPictureFree` で一度ずつ解放する。
        let bytes = unsafe {
            let imported = if has_alpha {
                WebPPictureImportRGBA(&mut picture, pixels.as_ptr(), stride as i32)
            } else {
                WebPPictureImportRGB(&mut picture, pixels.as_ptr(), stride as i32)
            };
            if imported == 0 {
                WebPPictureFree(&mut picture);
                return Err(failed("import"));
            }
            WebPMemoryWriterInit(writer.as_mut_ptr());
            picture.writer = Some(WebPMemoryWrite);
            picture.custom_ptr = writer.as_mut_ptr().cast();
            let ok = WebPEncode(&config, &mut picture);
            let writer = writer.assume_init_mut();
            let bytes = (ok != 0 && !writer.mem.is_null() && writer.size > 0).then(|| std::slice::from_raw_parts(writer.mem, writer.size).to_vec());
            let code = picture.error_code;
            WebPMemoryWriterClear(writer);
            WebPPictureFree(&mut picture);
            match bytes {
                Some(bytes) => bytes,
                None => return Err(failed(&format!("{:?}", code as WebPEncodingError))),
            }
        };
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};
    use testresult::TestResult;

    use super::*;

    /// 滑らかではない画素の並びで、品質の違いが大きさに出るようにする。
    fn noisy(width: u32, height: u32) -> DynamicImage {
        DynamicImage::ImageRgba8(RgbaImage::from_fn(width, height, |x, y| {
            let v = x.wrapping_mul(2654435761).wrapping_add(y.wrapping_mul(40503)) >> 8;
            Rgba([v as u8, (v >> 3) as u8, (v >> 5) as u8, 255])
        }))
    }

    #[test]
    fn encodes_a_webp_that_the_image_crate_reads_back_test() -> TestResult {
        let bytes = WebpEncoder::encode_lossy(&noisy(64, 32), 80)?;
        assert_eq!(image::guess_format(&bytes)?, image::ImageFormat::WebP);
        let decoded = image::load_from_memory(&bytes)?;
        assert_eq!((decoded.width(), decoded.height()), (64, 32));
        Ok(())
    }

    #[test]
    fn a_lower_quality_makes_a_smaller_file_test() -> TestResult {
        let image = noisy(128, 128);
        let low = WebpEncoder::encode_lossy(&image, 30)?;
        let high = WebpEncoder::encode_lossy(&image, 90)?;
        assert!(low.len() < high.len(), "{} < {}", low.len(), high.len());
        Ok(())
    }

    #[test]
    fn keeps_transparency_test() -> TestResult {
        let transparent = DynamicImage::ImageRgba8(RgbaImage::from_pixel(16, 16, Rgba([0, 0, 0, 0])));
        let decoded = image::load_from_memory(&WebpEncoder::encode_lossy(&transparent, 80)?)?.to_rgba8();
        assert_eq!(decoded.get_pixel(8, 8)[3], 0);
        Ok(())
    }

    #[test]
    fn encodes_an_image_without_alpha_test() -> TestResult {
        let opaque = DynamicImage::ImageRgb8(image::RgbImage::from_pixel(32, 16, image::Rgb([200, 40, 40])));
        let decoded = image::load_from_memory(&WebpEncoder::encode_lossy(&opaque, 90)?)?.to_rgb8();
        assert_eq!((decoded.width(), decoded.height()), (32, 16));
        let pixel = decoded.get_pixel(16, 8);
        assert!(pixel[0] > 180 && pixel[1] < 70 && pixel[2] < 70, "{pixel:?}");
        Ok(())
    }

    #[test]
    fn rejects_an_image_beyond_the_webp_limit_test() {
        let wide = DynamicImage::ImageRgba8(RgbaImage::new(WebpEncoder::MAX_DIMENSION + 1, 1));
        let error = WebpEncoder::encode_lossy(&wide, 80).unwrap_err();
        assert_eq!(*error.kind(), ErrorKind::InvalidInput);
    }
}
