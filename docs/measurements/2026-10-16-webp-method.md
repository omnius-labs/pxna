# Comparing WebP Compression Effort (method)

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

2026-10-16 (UTC). In [Measuring the Viewer's Display Image Latency](./2026-10-16-display-image-latency.md), reading and conversion in first generation was the main cause. We confirmed the contribution of WebP encoding within conversion and measured the effect of lowering libwebp's compression effort (`method`, 0 to 6). Each run's values are in the [aggregate JSON](./2026-10-16-webp-method.json).

- Preliminary measurement: we changed the configuration `[thumbnail] format` to JPEG and measured the same `--case initial` in one start. First generation went from 1.28 seconds to 0.61 seconds for local and from 1.45 seconds to 0.76 seconds for in-archive. We took this difference as the contribution of WebP encoding and investigated the encoder settings (JPEG responses are about 1.5 times larger, so we did not choose to change the format).
- Encoder sweep: we replaced `WebPEncodeRGBA` (the simple API, with method fixed at 4) with libwebp's advanced API that accepts settings, and for images shrunk to a long side of 4096, measured the median of 3 encodes with `method` from 0 to 6 at quality 90. We also tried `thread_level`, but the time did not change (the libwebp in this build is not parallelized), so we do not adopt it. PSNR is the RGB ratio against the shrunk original image. The temporary tests for the sweep are not committed.
- Comparison before (D1) and after (D2) implementation: under the same conditions as the earlier record (fixture, request, environment, 3 starts), we ran `initial`, `cached`, and `flip`. For D1 we use the values from the earlier record. D2 has the same measurement points as D1 except that `method` is fixed at 2.
- The order was D1 first and D2 after. They were not alternated.

## 2. Results

### 2.1 Encoder Sweep

Time is in seconds, size in bytes. PSNR is in dB.

| Image | method | 0 | 1 | 2 | 3 | 4 (default) | 5 | 6 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Synthetic (6000 × 4000 to 4096 × 2731) | Time | 0.344 | 0.441 | 0.466 | 0.819 | 0.813 | 0.958 | 2.275 |
| | Size | 4279538 | 4262584 | 4073634 | 4183534 | 4189092 | 4149666 | 4132418 |
| Sonoma (6016 × 6016 to 4096 × 4096) | Time | 0.171 | 0.232 | 0.195 | 0.511 | 0.522 | 0.550 | 0.560 |
| | Size | 262538 | 255532 | 195546 | 188072 | 191950 | 198970 | 203232 |
| | PSNR | 46.09 | 46.11 | 46.41 | 46.48 | 46.45 | 46.45 | 46.48 |
| Radial Sky Blue (same as above) | Time | 0.234 | 0.320 | 0.336 | 0.690 | 0.699 | 0.737 | 1.009 |
| | Size | 1236876 | 1210830 | 1102088 | 1054544 | 1057230 | 981342 | 933664 |
| | PSNR | 40.57 | 40.58 | 40.47 | 40.61 | 40.64 | 40.60 | 40.58 |
| iMac Silver (same as above) | Time | 0.223 | 0.299 | 0.357 | 0.763 | 0.767 | 0.825 | 1.338 |
| | Size | 1107474 | 1101500 | 1272754 | 1289832 | 1296178 | 1041620 | 1004322 |
| | PSNR | 42.08 | 42.09 | 42.39 | 42.25 | 42.27 | 41.79 | 41.90 |

Method 2 took 0.37 to 0.57 times the time of method 4, with a PSNR difference within 0.2 dB and a size difference of −3% to +4%. Methods 0 and 1 are faster still, but become 30% or more larger for Sonoma. Method 3 and above are slower than 2. Sonoma, Radial Sky Blue, and iMac Silver are macOS desktop images converted to JPEG (quality 92), and do not represent the distribution of photos.

### 2.2 First Generation (D1: method 4, D2: method 2)

The unit is seconds. Median / minimum / maximum of 15 runs per group. The response WebP is 4.19 MB (D1) and 4.07 MB (D2).

| Section | D1 local | D2 local | D1 in archive | D2 in archive |
| --- | --- | --- | --- | --- |
| Whole API | 1.258 / 1.174 / 1.522 | 0.896 / 0.840 / 1.372 | 1.382 / 1.330 / 1.618 | 1.082 / 1.000 / 1.511 |
| Reading and conversion | 1.215 / 1.161 / 1.241 | 0.853 / 0.827 / 0.896 | 1.176 / 1.164 / 1.197 | 0.836 / 0.827 / 0.903 |

D2's reading and conversion is about 0.35 seconds (about 30%) shorter than D1's. This nearly matches the encoding difference in the sweep (0.33 to 0.41 seconds). The remaining about 0.85 seconds of conversion is the total of decoding, shrinking, and encoding (in the sweep, decoding 0.1 to 0.26 seconds, shrinking 0.28 to 0.45 seconds, and method 2 encoding 0.2 to 0.47 seconds).

### 2.3 Waits in Consecutive Flipping

Under the same conditions as the earlier record, from the step's time until the display image arrives. 27 samples per column (9 steps excluding the first image × 3 runs).

| Group | Operation interval | D1 wait (median / min / max) | D2 wait (median / min / max) | Count that waited 0.2 s or more (D1 → D2) |
| --- | --- | --- | --- | --- |
| local | 1000 ms | 0.612 / 0.042 / 1.546 | 0.136 / 0.000 / 0.708 | 23 → 11 |
| local | 300 ms | 0.124 / 0.001 / 1.667 | 0.049 / 0.000 / 1.677 | 12 → 12 |
| in archive | 1000 ms | 0.793 / 0.000 / 1.372 | 0.361 / 0.001 / 0.892 | 23 → 19 |
| in archive | 300 ms | 0.108 / 0.000 / 1.994 | 0.097 / 0.000 / 1.778 | 12 → 12 |

The median wait at the 1-second interval fell from 0.61 seconds to 0.14 seconds for local and from 0.79 seconds to 0.36 seconds for in-archive. The 300 ms column is built so that the next operation waits until the current image arrives, so the count of waits does not change (when operations are faster than generation, the maximum wait stays around 1.7 seconds).

## 3. Conclusion

Lowering WebP encoding's compression effort from the default 4 to 2 made first generation about 30% shorter (from 1.26 seconds to 0.90 seconds for local), and lowered the median wait of flipping at 1-second intervals by about 80% (local). Image quality (PSNR) and size stayed within 0.2 dB and ±4% across 3 kinds of images. This change was adopted as the default of `WebpEncoder`.

The remaining slowness is the total of decoding (about 0.2 seconds), shrinking (0.3 to 0.45 seconds), and encoding (about 0.2 to 0.5 seconds), the stretching due to concurrently running generations, and prefetch covering only one image before and after. As for the size steps of display images (2048 and 4096), in this measurement a request of 1440 × 900 at pixel density 2 was also rounded up to 4096.

## 4. Limits

- The only image quality metric is PSNR, and we did not compare appearance (SSIM or visual inspection). The values are for quality 90, and do not apply to configurations that change `display_quality`.
- The sweep has 4 images, 3 of which are macOS desktop images. We did not confirm how the effect of method differs with the content of real photos.
- D1 and D2 were not alternated, and D1 was measured first. The influence of order and host variation cannot be removed.
- The preliminary measurement (switching to JPEG) was a single start with only 5 images, and the numbers are not in the aggregate JSON.
- The same limits as the earlier record remain (loopback, synthetic images, browser decoding and real network lines not measured).
