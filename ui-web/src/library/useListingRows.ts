import { useVirtualizer } from "@tanstack/react-virtual";
import { useEffect } from "react";

/// 読み込み済みの件数に左右されず、グリッドとリストの全体の高さを同じ規則で確保する。
export function useListingRows(
    totalCount: number | undefined,
    columns: number,
    height: number,
    scrollElement: HTMLDivElement | null,
) {
    const rowCount = Math.ceil((totalCount ?? 0) / columns);
    const virtualizer = useVirtualizer({
        count: rowCount,
        getScrollElement: () => scrollElement,
        estimateSize: () => height,
        overscan: 2,
    });
    // biome-ignore lint/correctness/useExhaustiveDependencies: height は変化時に再計測させる gate で、本体からは参照しない意図的な依存
    useEffect(() => {
        virtualizer.measure();
    }, [height, virtualizer]);
    return { rowCount, virtualizer };
}
