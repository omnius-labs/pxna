import { MIN_ZOOM } from "./viewer";

/// A flick is a horizontal movement of more than this many CSS px that ends within `FLICK_MS`.
export const FLICK_DISTANCE = 50;
export const FLICK_MS = 300;
/// A downward swipe is a downward movement of more than this many CSS px.
export const SWIPE_DISTANCE = 80;
/// A tap stays within this many CSS px and this many ms.
export const TAP_SLOP = 10;
export const TAP_MS = 300;
/// A double tap is two taps within this many ms and CSS px of each other.
export const DOUBLE_TAP_MS = 300;
export const DOUBLE_TAP_DISTANCE = 30;

export interface Point {
    x: number;
    y: number;
}

export interface PointerSample extends Point {
    id: number;
    /// Milliseconds on any one clock, such as `event.timeStamp`.
    time: number;
}

export type Gesture =
    | { kind: "flickLeft" }
    | { kind: "flickRight" }
    | { kind: "swipeDown" }
    | { kind: "doubleTap"; point: Point }
    /// `scale` is the change of the distance between the two fingers since the last pinch, `midpoint` is where their midpoint is now, and `shift` is how far it moved.
    | { kind: "pinch"; scale: number; midpoint: Point; shift: Point }
    | { kind: "pan"; delta: Point };

interface Track {
    start: PointerSample;
    last: PointerSample;
}

interface Pair {
    distance: number;
    midpoint: Point;
}

/// Recognizes touch gestures from the samples of pointer events. It keeps no timers and reads no clock, so the thresholds can be tested without a browser.
/// Flicks and the downward swipe are recognized only at the whole view (`zoom` of `MIN_ZOOM`), and a one-finger pan only while enlarged.
export interface GestureRecognizer {
    down(sample: PointerSample): void;
    move(sample: PointerSample, zoom: number): Gesture | undefined;
    up(sample: PointerSample, zoom: number): Gesture | undefined;
    cancel(id: number): void;
}

export function createGestureRecognizer(): GestureRecognizer {
    const tracks = new Map<number, Track>();
    // Set once a second finger touches, until every finger has lifted, so the rest of the touch is never a tap, a flick, or a pan.
    let multi = false;
    let pair: Pair | undefined;
    let lastTap: PointerSample | undefined;

    const measure = (): Pair | undefined => {
        const [a, b] = [...tracks.values()];
        if (!a || !b) return undefined;
        return {
            distance: Math.hypot(a.last.x - b.last.x, a.last.y - b.last.y),
            midpoint: { x: (a.last.x + b.last.x) / 2, y: (a.last.y + b.last.y) / 2 },
        };
    };

    const release = (id: number) => {
        tracks.delete(id);
        pair = undefined;
        if (tracks.size === 0) multi = false;
    };

    return {
        down(sample) {
            tracks.set(sample.id, { start: sample, last: sample });
            if (tracks.size >= 2) {
                multi = true;
                lastTap = undefined;
                pair = measure();
            }
        },

        move(sample, zoom) {
            const track = tracks.get(sample.id);
            if (!track) return undefined;
            const before = track.last;
            track.last = sample;
            if (tracks.size === 2) {
                const now = measure();
                const previous = pair;
                pair = now;
                if (!now || !previous || previous.distance === 0) return undefined;
                return {
                    kind: "pinch",
                    scale: now.distance / previous.distance,
                    midpoint: now.midpoint,
                    shift: { x: now.midpoint.x - previous.midpoint.x, y: now.midpoint.y - previous.midpoint.y },
                };
            }
            if (tracks.size === 1 && !multi && zoom > MIN_ZOOM) {
                return { kind: "pan", delta: { x: sample.x - before.x, y: sample.y - before.y } };
            }
            return undefined;
        },

        up(sample, zoom) {
            const track = tracks.get(sample.id);
            if (!track) return undefined;
            const counted = !multi;
            release(sample.id);
            if (!counted) {
                lastTap = undefined;
                return undefined;
            }
            const { start } = track;
            const dx = sample.x - start.x;
            const dy = sample.y - start.y;
            const elapsed = sample.time - start.time;
            if (zoom <= MIN_ZOOM) {
                if (elapsed < FLICK_MS && Math.abs(dx) > FLICK_DISTANCE && Math.abs(dx) > Math.abs(dy)) {
                    lastTap = undefined;
                    return { kind: dx < 0 ? "flickLeft" : "flickRight" };
                }
                if (dy > SWIPE_DISTANCE && dy > Math.abs(dx)) {
                    lastTap = undefined;
                    return { kind: "swipeDown" };
                }
            }
            if (Math.hypot(dx, dy) > TAP_SLOP || elapsed > TAP_MS) {
                lastTap = undefined;
                return undefined;
            }
            const previous = lastTap;
            if (
                previous &&
                sample.time - previous.time <= DOUBLE_TAP_MS &&
                Math.hypot(sample.x - previous.x, sample.y - previous.y) <= DOUBLE_TAP_DISTANCE
            ) {
                lastTap = undefined;
                return { kind: "doubleTap", point: { x: sample.x, y: sample.y } };
            }
            lastTap = sample;
            return undefined;
        },

        cancel(id) {
            release(id);
            lastTap = undefined;
        },
    };
}
