import { type KeyboardEvent, type PointerEvent, useRef } from "react";

import { WIDTH_KEY_STEP, type WidthRange } from "../library/panelWidth";

interface ResizeHandleProps {
    label: string;
    range: WidthRange;
    width: number;
    /// 端を右へ動かすと広がるか。ツリーの右端は広がり、情報パネルの左端は狭まる。
    grows: "right" | "left";
    onResize(width: number): void;
    /// ドラッグかキー操作が終わったときに、最終の幅で呼ぶ。
    onCommit(width: number): void;
}

/// パネルの端に置く、幅を変えるつまみ。ポインタのドラッグと、左右の矢印キーで動かす。
export function ResizeHandle({ label, range, width, grows, onResize, onCommit }: ResizeHandleProps) {
    const drag = useRef<{ startX: number; startWidth: number; last: number }>(undefined);
    const sign = grows === "right" ? 1 : -1;

    const move = (event: PointerEvent<HTMLDivElement>) => {
        if (!drag.current) return;
        const next = drag.current.startWidth + sign * (event.clientX - drag.current.startX);
        drag.current.last = next;
        onResize(next);
    };
    const end = () => {
        if (!drag.current) return;
        const { last } = drag.current;
        drag.current = undefined;
        onCommit(last);
    };
    const keyDown = (event: KeyboardEvent<HTMLDivElement>) => {
        const direction = event.key === "ArrowRight" ? 1 : event.key === "ArrowLeft" ? -1 : 0;
        if (direction === 0) return;
        event.preventDefault();
        const next = width + sign * direction * WIDTH_KEY_STEP;
        onResize(next);
        onCommit(next);
    };

    return (
        // biome-ignore lint/a11y/useSemanticElements: 幅を変えるつまみは、フォーカスできる separator として扱う（WAI-ARIA の window splitter）
        <div
            role="separator"
            aria-orientation="vertical"
            aria-label={label}
            aria-valuemin={range.min}
            aria-valuemax={range.max}
            aria-valuenow={width}
            tabIndex={0}
            className="w-1 shrink-0 cursor-col-resize touch-none bg-border/0 hover:bg-border focus-visible:bg-ring"
            onPointerDown={(event) => {
                event.currentTarget.setPointerCapture?.(event.pointerId);
                drag.current = { startX: event.clientX, startWidth: width, last: width };
            }}
            onPointerMove={move}
            onPointerUp={end}
            onPointerCancel={end}
            onKeyDown={keyDown}
        />
    );
}
