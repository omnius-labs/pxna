# The Player Does Not Report an Ended Video Session When the Browser Plays HLS Itself

**Severity: Low** (the owner sees the general playback failure instead of the reason; playback is not affected otherwise)

Investigated 2026-11-28 / Target commit `7088a63`

One item of [issues.md](../issues.md).

## Symptom

When the daemon ends a video session to make way for a newer one, the web client's player is meant to say that playback ended because another video started.
In a browser that plays HLS itself, such as Chromium 158, it shows "Could not play the video. Close it and open it again." instead, and nothing is logged.
The reason is shown only where the player uses hls.js.

## Location

- [VideoPlayer.tsx#L70](../../ui-web/src/components/VideoPlayer.tsx#L70): the browser's own HLS gets the playlist URL as `src`.
- [VideoPlayer.tsx#L78](../../ui-web/src/components/VideoPlayer.tsx#L78): only the hls.js error handler passes the HTTP status to `playbackFailure`.
- [VideoPlayer.tsx#L130](../../ui-web/src/components/VideoPlayer.tsx#L130): the element's `error` event always shows the general failure.

## Cause

A failed segment request of the browser's own HLS reaches the page only as the element's `error` event, which carries no HTTP status ([knowledge](../knowledge/chromium-native-hls.md)).
The player therefore cannot tell the `410` of an ended session from any other failure.

## Impact

Only the message differs; the daemon answers `410` with `SessionEvicted` as the contract says, and closing and opening the video starts a new session.
It shows when two videos are played at once with `max_concurrent_transcodes` at 1, for example on two devices.

## Remedy

On the element's `error` event in the browser's own HLS, read the session's playlist once and pass its status to `playbackFailure`, so a `410` shows the reason.
A test of `VideoPlayer` that fires the element's `error` with the playlist answering `410` keeps it fixed.
