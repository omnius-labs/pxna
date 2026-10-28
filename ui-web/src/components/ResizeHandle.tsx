import { type KeyboardEvent, type PointerEvent, useRef } from "react";

import { WIDTH_KEY_STEP, type WidthRange } from "../library/panelWidth";

interface ResizeHandleProps {
    label: string;
    range: WidthRange;
    width: number;
    /// つまみの向き。縦（既定）は幅を、横は高さを変える。
    orientation?: "vertical" | "horizontal";
    /// 端を右へ動かすと広がるか。ツリーの右端は広がり、情報パネルの左端は狭まる。
    grows: "right" | "left" | "down";
    onResize(width: number): void;
    /// ドラッグかキー操作が終わったときに、最終の幅で呼ぶ。
    onCommit(width: number): void;
}

/// パネルの端に置く、幅を変えるつまみ。ポインタのドラッグと、左右の矢印キーで動かす。
export function ResizeHandle({
    label,
    range,
    width,
    grows,
    orientation = "vertical",
    onResize,
    onCommit,
}: ResizeHandleProps) {
    const drag = useRef<{ start: number; startWidth: number; last: number }>(undefined);
    const sign = grows === "left" ? -1 : 1;
    const horizontal = orientation === "horizontal";
    const axis = (event: { clientX: number; clientY: number }) => (horizontal ? event.clientY : event.clientX);

    const move = (event: PointerEvent<HTMLDivElement>) => {
        if (!drag.current) return;
        const next = drag.current.startWidth + sign * (axis(event) - drag.current.start);
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
        const [more, less] = horizontal ? ["ArrowDown", "ArrowUp"] : ["ArrowRight", "ArrowLeft"];
        const direction = event.key === more ? 1 : event.key === less ? -1 : 0;
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
            aria-orientation={orientation}
            aria-label={label}
            aria-valuemin={range.min}
            aria-valuemax={range.max}
            aria-valuenow={width}
            tabIndex={0}
            className={`shrink-0 touch-none bg-border/0 hover:bg-border focus-visible:bg-ring ${horizontal ? "h-1 cursor-row-resize" : "w-1 cursor-col-resize"}`}
            onPointerDown={(event) => {
                event.currentTarget.setPointerCapture?.(event.pointerId);
                drag.current = { start: axis(event), startWidth: width, last: width };
            }}
            onPointerMove={move}
            onPointerUp={end}
            onPointerCancel={end}
            onKeyDown={keyDown}
        />
    );
}
