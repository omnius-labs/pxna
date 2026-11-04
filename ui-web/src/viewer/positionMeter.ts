/// The largest number of bars. With fewer Entries than this, each Entry has its own bar.
export const METER_MAX_BARS = 48;

const METER_BASE_HEIGHT = 8;
const METER_PEAK_HEIGHT = 16;
/// How many bars away from the open position a bar still grows.
const METER_SPREAD = 4;

export function meterBarCount(count: number): number {
    return Math.min(Math.max(0, count), METER_MAX_BARS);
}

/// The bar holding a position counted from the start of the range.
export function meterBarAt(position: number, count: number): number {
    const bars = meterBarCount(count);
    if (bars === 0) return 0;
    return Math.min(bars - 1, Math.max(0, Math.floor((position * bars) / count)));
}

/// The position counted from the start of the range that a bar moves to: the middle of the positions it holds.
export function meterPositionOf(bar: number, count: number): number {
    const bars = meterBarCount(count);
    if (bars === 0) return 0;
    const from = Math.ceil((bar * count) / bars);
    const to = Math.ceil(((bar + 1) * count) / bars) - 1;
    return Math.floor((from + to) / 2);
}

/// The height (CSS px) of a bar. It grows as it nears the bar of the open position.
export function meterBarHeight(bar: number, currentBar: number): number {
    const nearness = Math.max(0, 1 - Math.abs(bar - currentBar) / METER_SPREAD);
    return METER_BASE_HEIGHT + METER_PEAK_HEIGHT * nearness;
}
