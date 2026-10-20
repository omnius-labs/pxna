# Measuring the Viewer's Display Image Latency

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

2026-10-16 (UTC). Following [comparison procedure §10](./browse-performance-protocol.md#10-measuring-display-image-latency), we measured the sections of display images with a build of the pre-change daemon to which only measurement points were added. Countermeasures are not included. Each run's values are in the [aggregate JSON](./2026-10-16-display-image-latency.json).

- Build: a release build of `5e1d4c5` of `main` with only the measurement points `thumbnail_generation`, `thumbnail_materialize`, `thumbnail_convert`, and `thumbnail_store` added (the SHA-256 is recorded in the JSON).
- Environment: Apple M4 (10 cores), macOS (Darwin 25.6.0), the fixture on APFS. The OS file cache was not controlled.
- Fixture: copies of a 6000 × 4000 JPEG (about 13.5 MB, a synthetic image generated with ffmpeg). Local is placed in `/local` and `/flip`, and the archive is inside one uncompressed ZIP (`/arch.zip`).
- Request: assuming a viewer screen of 1440 × 900 at pixel density 2, `variant=display`, width 2880, height 1800, `resize=fit` (the same as the viewer's `displayRequestSize`). The response is WebP (about 4.19 MB).
- Settings: thumbnails use the defaults (`display_sizes` of 2048 and 4096, `display_quality` of 90, `max_concurrent_images` of 4). The timeline's periodic aggregation was stopped, and we measured after waiting for the startup aggregation to finish.
- Procedure: immediately after restarting the daemon, run `initial`, `cached`, and `flip` in this order. This was repeated 3 times (there is no alternating comparison target).

## 2. Results

### 2.1 First Generation (from Cache Miss to Response)

The unit is seconds. Median / minimum / maximum of 15 runs per group (5 files × 3 starts). The API is measured until the body is read to the end.

| Section | local | in archive |
| --- | --- | --- |
| Whole API | 1.258 / 1.174 / 1.522 | 1.382 / 1.330 / 1.618 |
| Generation (`thumbnail_generation`) | 1.257 / 1.172 / 1.520 | 1.380 / 1.328 / 1.616 |
| Extracting the original image (`thumbnail_materialize`) | 0.000 | 0.165 / 0.139 / 0.207 |
| Reading and conversion (`thumbnail_convert`) | 1.215 / 1.161 / 1.241 | 1.176 / 1.164 / 1.197 |
| Saving to the cache (`thumbnail_store`) | 0.030 / 0.009 / 0.155 | 0.032 / 0.010 / 0.273 |

Conversion accounts for about 97% (local) and about 85% (in archive) of generation. For in-archive images, about 0.17 seconds of extraction is nearly all of the difference from local. The difference between the whole API and generation is around 2 milliseconds, and there was no wait for a slot to generate.

### 2.2 Cache Hit and Swapping to the Original Image

The unit is seconds. Median / minimum / maximum of 75 runs per group (5 files × 5 times × 3 starts).

| Section | local | in archive |
| --- | --- | --- |
| Response for a display image cache hit | 0.003 / 0.002 / 0.008 | 0.006 / 0.001 / 0.014 |
| Fetching the original image (`files/content`, 13.5 MB) | 0.003 / 0.003 / 0.138 | 0.005 / 0.003 / 0.338 |

All 150 hits were hits of `thumbnail_image_cache`. The time for swapping is until the body is read to the end over loopback, and does not include browser decoding and painting or a real network line.

### 2.3 Consecutive Flipping (Prefetch Requests and Waits)

As in the viewer, opening an image requests the display images of that image and the one before and after it. Each column is 3 runs advancing through 10 images (9 steps excluding the first image × 3 runs = 27 samples). The unit is seconds, and "wait" is from the step's time until the display image arrives. The next operation is performed after the current image arrives and at or after the scheduled time.

| Group | Operation interval | Wait for the first image (median) | Waits afterward (median / min / max) | Count that waited 0.2 s or more | Duration of requests afterward (median) |
| --- | --- | --- | --- | --- | --- |
| local | 1000 ms | 1.517 | 0.612 / 0.042 / 1.546 | 23 / 27 | 1.551 |
| local | 300 ms | 1.454 | 0.124 / 0.001 / 1.667 | 12 / 27 | 1.442 |
| in archive | 1000 ms | 1.769 | 0.793 / 0.000 / 1.372 | 23 / 27 | 1.728 |
| in archive | 300 ms | 1.819 | 0.108 / 0.000 / 1.994 | 12 / 27 | 1.758 |

The prefetch requests had all started within the previous step. Even so, as concurrently running generations increase, each generation takes longer. The median of generation during consecutive flipping (`thumbnail_production`) is 1.55 to 1.68 seconds, about 0.3 to 0.4 seconds longer than a single first generation (1.26 seconds). The wait for a slot to generate did not appear within the measured range, because `thumbnail_generation` and `thumbnail_production` nearly matched. In the 300 ms column, waits appear every other step because the next operation waits until the current image arrives.

## 3. Section That Is the Main Cause

The main cause of the perceived slowness is the reading and conversion (about 1.2 seconds, 85 to 97% of generation) in generating display images that are not in the cache. In consecutive flipping, because prefetching covers only one image before and after, the generation time, stretched to about 1.6 seconds by concurrently running generations, exceeds the operation interval (1 second), and most steps wait 0.4 to 1.5 seconds.

- A cache hit (a few ms) and fetching the original image over loopback (a few ms) are not the main cause. However, we did not measure transferring 13.5 MB over a real network line or browser decoding, so the perceived feel of swapping needs to be checked separately.
- We did not measure the breakdown of conversion (decoding, shrinking, encoding). Which part of shrinking a 24-megapixel JPEG is large needs additional measurement.

### 3.1 Candidate Measures and the Corresponding Values

Implementation is done in a later plan. The values were obtained with a fixed synthetic image.

| Candidate | Corresponding value |
| --- | --- |
| Prefetch deeper than one image before and after (2 or more in the direction of travel) | The waits in consecutive flipping concentrate on steps the prefetch did not reach (23 / 27 at 1000 ms). Generation takes 1.6 seconds, so at a 1-second interval 2 or more images ahead are needed |
| Speed up conversion (reduced-size decoding, encoder settings) | Conversion is 85 to 97% of generation (about 1.2 seconds) |
| Prioritize generating the current image over prefetch | When running concurrently, one generation stretched from 1.26 seconds to about 1.6 seconds |
| Generate in advance (background generation when a folder is opened) | First generation is 1.2 to 1.4 seconds, and a hit is a few ms. The first image, which prefetch does not reach, takes 1.4 to 1.8 seconds |

### 3.2 Afterward

The WebP encoding in conversion accounted for a large share, so we lowered the compression effort and measured again. The result is in [Comparing WebP Compression Effort](./2026-10-16-webp-method.md). The values in this document are from before that change (method 4).

## 4. Limits

- The images are copies of one kind of synthetic image (6000 × 4000, about 13.5 MB), and do not represent the real distribution of photo content, sizes, and formats (PNG, HEIC, video frames, and so on). No extrapolation to real data is made.
- The measurement is HTTP over loopback and does not include browser decoding, painting, or a real network line. We do not know the contribution of these to the perceived slowness of display images.
- We compared the same build over 3 starts, so the influence of order and host variation cannot be removed. The OS file cache was not controlled.
- `flip` measured the operation interval and prefetch depth in one way (one image before and after). We did not measure the distribution of users' actual operation intervals.
- The images in the archive are in an uncompressed ZIP. We did not measure the cost of extracting from compressed archives or 7z.
