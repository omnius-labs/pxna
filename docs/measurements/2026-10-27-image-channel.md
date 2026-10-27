# Switching the Web Client to the Image Channel

Back to the [measurement list](../measurements.md).

## 1. Subject and Conditions

2026-10-27 (UTC). We switched every image want of the web client to the image channel and measured the duplicate external decodes and the answers after a reload. The change is the working tree of the image channel client plan on `d4030c6`.

- Build: the daemon was built with Rust 1.96.0 in the development profile; the web client was a production build of the same working tree.
- Environment: Debian GNU/Linux 13 (trixie) x86_64, 10 cores, 8 GiB. The daemon ran on the host with the ffmpeg and heif-dec tools of the container image. The OS file cache was not controlled.
- Fixture: 12 files in one folder, made by copying two iPhone 15 Plus HEIC files (`IMG_5985.heic`, 3.4 MB, and `IMG_6816.heic`, 1.3 MB) six times each under distinct names. Each copy is its own source, so the counts below are per path.
- Browser: Headless Chrome 156, viewport 1280 × 720, device pixel ratio 1.
- Procedure: browse `/browse/photos` in the grid, open the viewer on `IMG_01.heic`, `IMG_02.heic`, and `IMG_03.heic` with the next button, reload `/browse/photos`, and open the viewer on `IMG_01.heic` again. Performance logging was on with JSON output.

## 2. Results

### 2.1 External Decodes

The log records one `thumbnail_external_decode` span under the request that started the source job. A request that another request's job serves shows only its `thumbnail_job_wait`, so the span count is the number of external decodes, not the number of images.

| Count | Value |
| --- | ---: |
| Sources browsed | 12 |
| `thumbnail_external_decode` spans | 16 |
| Sources decoded twice or more | 4 |
| Two external decodes of one source overlapping in time | 0 |
| `thumbnail_request` spans | 29 |
| Thumbnail requests | 25 |
| Display requests | 4 |

Two decodes of one source are sequential, at different size steps, and none overlap another decode of the same source. This is the condition of the plan.

### 2.2 After the Reload

The client keeps received images in IndexedDB and wants every image again with its `ETag`. A file that did not change is answered with `unchanged` and no image frame.

| Count | Value |
| --- | ---: |
| `thumbnail_image` client timings | 50 |
| Generated (`cached=false`) | 25 |
| Answered `unchanged` after the reload (`cached=true`) | 25 |

The 25 answers marked cached are the tiles and filmstrip thumbnails that the reload wanted again with their stored `ETag`; no image bytes were sent for them. The 25 generated answers are the images the first visit made.

### 2.3 HTTP Requests

The browser's request log contains no request to `/api/v1/libraries/photos/thumbnails` while browsing the grid, opening the viewer, loading the filmstrip, or after the reload. The images arrive over the WebSocket at `/api/v1/image-channel`, and the tiles and filmstrip appear from IndexedDB before the `unchanged` answers.

## 3. Limits

The fixture repeats two real HEIC files, so the decode times for different names use the same pixels and the OS file cache is warm after the first read. The absolute times are not recorded here; the counts of overlapping decodes and `unchanged` answers are the subject. This is one run, not a distribution. The 25 `unchanged` answers depend on the folder's files not changing during the run, which they did not.
