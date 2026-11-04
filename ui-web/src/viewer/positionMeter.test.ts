import { expect, it } from "vitest";
import { METER_MAX_BARS, meterBarAt, meterBarCount, meterBarHeight, meterPositionOf } from "./positionMeter";

it("gives each Entry a bar until the bars run out", () => {
    expect(meterBarCount(0)).toBe(0);
    expect(meterBarCount(3)).toBe(3);
    expect(meterBarCount(10_000)).toBe(METER_MAX_BARS);
});

it("maps both ends of the range to the first and last bars", () => {
    expect(meterBarAt(0, 10_000)).toBe(0);
    expect(meterBarAt(9_999, 10_000)).toBe(METER_MAX_BARS - 1);
});

it("maps a bar to a position inside that bar", () => {
    for (const count of [1, 3, 48, 49, 10_000]) {
        for (let bar = 0; bar < meterBarCount(count); bar += 1) {
            expect(meterBarAt(meterPositionOf(bar, count), count)).toBe(bar);
        }
    }
});

it("grows the bars near the open position only", () => {
    expect(meterBarHeight(5, 5)).toBe(24);
    expect(meterBarHeight(6, 5)).toBeLessThan(24);
    expect(meterBarHeight(6, 5)).toBeGreaterThan(8);
    expect(meterBarHeight(20, 5)).toBe(8);
});
