import { describe, expect, it } from "vitest";

import {
    formatBytes,
    formatCamera,
    formatDimensions,
    formatDuration,
    formatExposureTime,
    formatFNumber,
    formatFocalLength,
    formatTakenAt,
} from "./format";

describe("formatBytes", () => {
    it.each([
        [0, "0 B"],
        [1023, "1023 B"],
        [1024, "1.0 KB"],
        [1536, "1.5 KB"],
        [5 * 1024 * 1024, "5.0 MB"],
        [3 * 1024 ** 4, "3.0 TB"],
    ])("formats %d bytes as %s", (bytes, text) => {
        expect(formatBytes(bytes)).toBe(text);
    });
});

describe("メディア情報の整形", () => {
    it("寸法は、回転が 90 度か 270 度のときだけ、幅と高さを入れ替える", () => {
        expect(formatDimensions(64, 32)).toBe("64 × 32");
        expect(formatDimensions(64, 32, 0)).toBe("64 × 32");
        expect(formatDimensions(64, 32, 90)).toBe("32 × 64");
        expect(formatDimensions(64, 32, 180)).toBe("64 × 32");
        expect(formatDimensions(64, 32, 270)).toBe("32 × 64");
    });

    it("長さは、1 時間未満を「分:秒」、以上を「時:分:秒」にし、小数の秒を四捨五入する", () => {
        expect(formatDuration(0)).toBe("0:00");
        expect(formatDuration(2)).toBe("0:02");
        expect(formatDuration(59.6)).toBe("1:00");
        expect(formatDuration(754)).toBe("12:34");
        expect(formatDuration(3725)).toBe("1:02:05");
    });

    it("露出時間は、1 秒未満を分数、1 秒以上を秒数にする", () => {
        expect(formatExposureTime(0.004)).toBe("1/250 s");
        expect(formatExposureTime(0.5)).toBe("1/2 s");
        expect(formatExposureTime(1)).toBe("1 s");
        expect(formatExposureTime(2.5)).toBe("2.5 s");
    });

    it("絞り値と焦点距離は、不要な小数を付けない", () => {
        expect(formatFNumber(2.8)).toBe("f/2.8");
        expect(formatFNumber(2)).toBe("f/2");
        expect(formatFocalLength(24)).toBe("24 mm");
        expect(formatFocalLength(5.7)).toBe("5.7 mm");
    });

    it("撮影日時は、秒を除いて、日付と時刻を空白で区切る", () => {
        expect(formatTakenAt("2026-04-05T14:32:10")).toBe("2026-04-05 14:32");
        expect(formatTakenAt("Unknown")).toBe("Unknown");
    });

    it("カメラは、モデルがメーカー名の最初の語で始まるときは重ねず、片方だけならそれを返す", () => {
        expect(formatCamera("Apple", "iPhone 17 Pro")).toBe("Apple iPhone 17 Pro");
        expect(formatCamera("Canon", "Canon EOS R5")).toBe("Canon EOS R5");
        expect(formatCamera("NIKON CORPORATION", "NIKON Z 6")).toBe("NIKON Z 6");
        expect(formatCamera("FUJIFILM", "X-T5")).toBe("FUJIFILM X-T5");
        expect(formatCamera("Sony", undefined)).toBe("Sony");
        expect(formatCamera(undefined, "X100V")).toBe("X100V");
        expect(formatCamera(undefined, undefined)).toBeUndefined();
    });
});
