# Chromium 158 Plays HLS in a `<video>` Element Without Media Source Extensions

**Source: Verified** (Chromium 158.0.0.0, headless, as bundled with Playwright / 2026-11-28; a session of the web client / 2026-11-28)

One item of [knowledge.md](../knowledge.md).

## Fact

In Chromium 158, `HTMLVideoElement.canPlayType("application/vnd.apple.mpegurl")` answers `"maybe"`, and a `<video>` whose `src` is an HLS playlist plays it.
The element then loads the playlist and the segments itself, and a failed segment request reaches the page only as the element's `error` event, without the HTTP status.

## Basis

On 2026-11-28, the web client opened an MPEG-4 Part 2 video in an AVI file, which the browser cannot play directly, in headless Chromium 158.0.0.0.
`canPlayType` answered `"maybe"` for the HLS type, the player set the playlist as `src` without loading hls.js, and the video played to 7.4 seconds.
After the daemon answered a segment with `410`, hls.js reported no error, because it was not running, and the element's `error` event fired.

## Implication

The web client's player chooses the browser's own HLS whenever `canPlayType` accepts it ([Video Playback Design §5.4](../design/video.md#54-concurrency)), so in Chromium it does not see the status of a session the daemon ended.
