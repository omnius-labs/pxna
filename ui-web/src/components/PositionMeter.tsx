import { t } from "i18next";
import { meterBarAt, meterBarCount, meterBarHeight, meterPositionOf } from "../viewer/positionMeter";

interface PositionMeterProps {
    /// The first position of the range the meter represents.
    first: number;
    /// The number of positions in that range.
    count: number;
    /// The open Entry's position.
    index: number;
    /// Moves to a position.
    onSeek: (index: number) => void;
}

/// A row of thin bars at the bottom edge of the viewer. The bars represent proportions of the range, and pressing one moves there.
export function PositionMeter({ first, count, index, onSeek }: PositionMeterProps) {
    const bars = meterBarCount(count);
    const current = meterBarAt(index - first, count);
    return (
        <nav aria-label={t("positionMeter.label")} className="flex h-10 shrink-0 items-end justify-center pb-2">
            {Array.from({ length: bars }, (_, bar) => {
                const percent = Math.round(((bar + 0.5) * 100) / bars);
                return (
                    <button
                        // The bars are fixed slots of the range, so their order never changes.
                        // biome-ignore lint/suspicious/noArrayIndexKey: the index is the identity of a bar
                        key={bar}
                        type="button"
                        aria-label={t("positionMeter.seek", { percent })}
                        aria-current={bar === current ? "true" : undefined}
                        // Tab stops only at the open position, as in the filmstrip.
                        tabIndex={bar === current ? 0 : -1}
                        className="group flex h-full w-2 items-end justify-center rounded-sm outline-none focus-visible:ring-2 focus-visible:ring-white"
                        onClick={() => onSeek(first + meterPositionOf(bar, count))}
                    >
                        <span
                            className={`block w-px transition-[height] group-hover:bg-white ${bar === current ? "bg-white" : "bg-white/40"}`}
                            style={{ height: meterBarHeight(bar, current) }}
                        />
                    </button>
                );
            })}
        </nav>
    );
}
