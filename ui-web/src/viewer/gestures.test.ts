import { describe, expect, it } from "vitest";

import { createGestureRecognizer, type Gesture, type PointerSample } from "./gestures";

const sample = (id: number, time: number, x: number, y: number): PointerSample => ({ id, time, x, y });

/// Puts one finger down at (0, 0) at time 0, moves it to the end point, and lifts it at `end.time`.
function stroke(end: { time: number; x: number; y: number }, zoom = 1, from = { x: 0, y: 0 }) {
    const recognizer = createGestureRecognizer();
    recognizer.down(sample(1, 0, from.x, from.y));
    recognizer.move(sample(1, end.time / 2, (from.x + end.x) / 2, (from.y + end.y) / 2), zoom);
    return recognizer.up(sample(1, end.time, end.x, end.y), zoom);
}

describe("flick", () => {
    it("moves to the next image on a leftward flick and the previous on a rightward flick", () => {
        expect(stroke({ time: 100, x: -120, y: 5 })).toEqual({ kind: "flickLeft" });
        expect(stroke({ time: 100, x: 120, y: -5 })).toEqual({ kind: "flickRight" });
    });

    it("needs more than 50 px within less than 300 ms", () => {
        expect(stroke({ time: 100, x: -51, y: 0 })).toEqual({ kind: "flickLeft" });
        expect(stroke({ time: 100, x: -50, y: 0 })).toBeUndefined();
        expect(stroke({ time: 299, x: -80, y: 0 })).toEqual({ kind: "flickLeft" });
        expect(stroke({ time: 300, x: -80, y: 0 })).toBeUndefined();
    });

    it("is not a flick when the movement is more vertical than horizontal", () => {
        expect(stroke({ time: 100, x: 60, y: 70 })).toBeUndefined();
        expect(stroke({ time: 100, x: 60, y: -70 })).toBeUndefined();
    });

    it("is not recognized while enlarged", () => {
        expect(stroke({ time: 100, x: -120, y: 0 }, 1.5)).toBeUndefined();
    });
});

describe("downward swipe", () => {
    it("needs a downward movement of more than 80 px, however slow", () => {
        expect(stroke({ time: 100, x: 3, y: 81 })).toEqual({ kind: "swipeDown" });
        expect(stroke({ time: 900, x: 0, y: 200 })).toEqual({ kind: "swipeDown" });
        expect(stroke({ time: 100, x: 0, y: 80 })).toBeUndefined();
    });

    it("is not recognized upward, while more horizontal, or while enlarged", () => {
        expect(stroke({ time: 100, x: 0, y: -200 })).toBeUndefined();
        expect(stroke({ time: 900, x: 150, y: 100 })).toBeUndefined();
        expect(stroke({ time: 100, x: 0, y: 200 }, 2)).toBeUndefined();
    });
});

describe("double tap", () => {
    const tap = (
        recognizer: ReturnType<typeof createGestureRecognizer>,
        time: number,
        x: number,
        y: number,
        zoom = 1,
    ) => {
        recognizer.down(sample(1, time, x, y));
        return recognizer.up(sample(1, time + 40, x, y), zoom);
    };

    it("is two taps within 300 ms and 30 px, reported at the second tap", () => {
        const recognizer = createGestureRecognizer();
        expect(tap(recognizer, 0, 100, 100)).toBeUndefined();
        expect(tap(recognizer, 200, 120, 100)).toEqual({ kind: "doubleTap", point: { x: 120, y: 100 } });
    });

    it("is recognized while enlarged too", () => {
        const recognizer = createGestureRecognizer();
        tap(recognizer, 0, 100, 100, 3);
        expect(tap(recognizer, 200, 100, 100, 3)).toEqual({ kind: "doubleTap", point: { x: 100, y: 100 } });
    });

    it("is not recognized when the taps are too late or too far apart", () => {
        const late = createGestureRecognizer();
        tap(late, 0, 100, 100);
        expect(tap(late, 400, 100, 100)).toBeUndefined();
        const far = createGestureRecognizer();
        tap(far, 0, 100, 100);
        expect(tap(far, 100, 131, 100)).toBeUndefined();
    });

    it("does not make a third tap a second double tap", () => {
        const recognizer = createGestureRecognizer();
        tap(recognizer, 0, 100, 100);
        expect(tap(recognizer, 100, 100, 100)).toBeDefined();
        expect(tap(recognizer, 200, 100, 100)).toBeUndefined();
    });

    it("does not count a touch that moved or was held", () => {
        const recognizer = createGestureRecognizer();
        tap(recognizer, 0, 100, 100);
        recognizer.down(sample(1, 100, 100, 100));
        expect(recognizer.up(sample(1, 140, 115, 100), 1)).toBeUndefined();
        expect(tap(recognizer, 200, 100, 100)).toBeUndefined();
        recognizer.down(sample(1, 300, 100, 100));
        expect(recognizer.up(sample(1, 700, 100, 100), 1)).toBeUndefined();
    });
});

describe("pinch", () => {
    function twoFingers() {
        const recognizer = createGestureRecognizer();
        recognizer.down(sample(1, 0, 100, 200));
        recognizer.down(sample(2, 0, 300, 200));
        return recognizer;
    }

    it("reports the change of the finger distance and the movement of the midpoint", () => {
        const recognizer = twoFingers();
        expect(recognizer.move(sample(1, 10, 50, 200), 1)).toEqual<Gesture>({
            kind: "pinch",
            scale: 1.25,
            midpoint: { x: 175, y: 200 },
            shift: { x: -25, y: 0 },
        });
        expect(recognizer.move(sample(2, 20, 350, 220), 1)).toMatchObject({
            kind: "pinch",
            midpoint: { x: 200, y: 210 },
            shift: { x: 25, y: 10 },
        });
        const last = recognizer.move(sample(2, 30, 250, 220), 1);
        expect(last?.kind === "pinch" && last.scale).toBeCloseTo(Math.hypot(200, 20) / Math.hypot(300, 20), 5);
    });

    it("recognizes nothing after the fingers lift, and does not pan with the remaining finger", () => {
        const recognizer = twoFingers();
        recognizer.move(sample(1, 10, 50, 200), 1.5);
        expect(recognizer.up(sample(1, 20, 50, 200), 1.5)).toBeUndefined();
        expect(recognizer.move(sample(2, 30, 400, 200), 1.5)).toBeUndefined();
        expect(recognizer.up(sample(2, 40, 400, 200), 1)).toBeUndefined();
    });

    it("does not end a pinch in a flick, a swipe, or a tap", () => {
        const recognizer = createGestureRecognizer();
        recognizer.down(sample(1, 0, 0, 0));
        recognizer.down(sample(2, 5, 10, 0));
        expect(recognizer.up(sample(2, 10, 10, 0), 1)).toBeUndefined();
        expect(recognizer.up(sample(1, 20, 0, 0), 1)).toBeUndefined();
        recognizer.down(sample(1, 100, 0, 0));
        expect(recognizer.up(sample(1, 140, 0, 0), 1)).toBeUndefined();
    });
});

describe("pan", () => {
    it("reports each movement of one finger while enlarged", () => {
        const recognizer = createGestureRecognizer();
        recognizer.down(sample(1, 0, 100, 100));
        expect(recognizer.move(sample(1, 10, 90, 105), 2)).toEqual({ kind: "pan", delta: { x: -10, y: 5 } });
        expect(recognizer.move(sample(1, 20, 80, 95), 2)).toEqual({ kind: "pan", delta: { x: -10, y: -10 } });
        expect(recognizer.up(sample(1, 30, 80, 95), 2)).toBeUndefined();
    });

    it("is not reported at the whole view", () => {
        const recognizer = createGestureRecognizer();
        recognizer.down(sample(1, 0, 100, 100));
        expect(recognizer.move(sample(1, 10, 90, 105), 1)).toBeUndefined();
    });

    it("forgets a cancelled touch", () => {
        const recognizer = createGestureRecognizer();
        recognizer.down(sample(1, 0, 100, 100));
        recognizer.cancel(1);
        expect(recognizer.move(sample(1, 10, 90, 105), 2)).toBeUndefined();
        expect(recognizer.up(sample(1, 20, 0, 300), 1)).toBeUndefined();
    });
});
