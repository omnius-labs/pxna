import { t } from "i18next";
import { type KeyboardEvent, type MouseEvent, useState, useSyncExternalStore } from "react";
import type { ImageWants } from "../images/wants";
import { isCut, useClipboard } from "../library/clipboard";
import { type Entry, entryKind } from "../library/entries";
import { PLACE_HEIGHT } from "../library/grid";
import type { DisplaySet } from "../library/preferences";
import { useFlipbook } from "../thumbnails/useFlipbook";
import { EntryIcon } from "./EntryIcon";

interface TileProps {
    entry: Entry;
    place?: string;
    wants: ImageWants;
    libraryId: string;
    thumbnailSize: number;
    /// タイルの表示の一辺（CSS ピクセル）。
    tileSide: number;
    shape: DisplaySet["thumbnailShape"];
    index: number;
    selected: boolean;
    focusable: boolean;
    onClick: (event: MouseEvent, index: number) => void;
    onDoubleClick: () => void;
    onKeyDown: (event: KeyboardEvent, index: number) => void;
}

export function Tile({
    entry,
    place,
    wants,
    libraryId,
    thumbnailSize: size,
    tileSide,
    shape,
    index,
    selected,
    focusable,
    onClick,
    onDoubleClick,
    onKeyDown,
}: TileProps) {
    const clipboard = useClipboard();
    const cut = isCut(clipboard.contents, libraryId, entry.path);
    const kind = entryKind(entry);
    const thumbnail = useSyncExternalStore(wants.subscribe, () => wants.getState(entry.path));
    // 動画のタイルにマウスを重ねている間は、スプライトシートのフレームを順に切り替える。
    const [hovering, setHovering] = useState(false);
    const flipbook = useFlipbook(hovering && kind === "video", libraryId, entry.path, size, shape, tileSide);
    return (
        <div
            role="option"
            aria-selected={selected}
            data-index={index}
            data-kind={kind}
            tabIndex={focusable ? 0 : -1}
            title={entry.name}
            className={`flex flex-col outline-none ${selected ? "rounded bg-accent text-accent-foreground" : ""} focus-visible:ring-2 focus-visible:ring-ring`}
            style={{ width: tileSide, opacity: cut ? 0.4 : undefined }}
            onClick={(event) => onClick(event, index)}
            onDoubleClick={onDoubleClick}
            onKeyDown={(event) => onKeyDown(event, index)}
        >
            {/* biome-ignore lint/a11y/noStaticElementInteractions: ホバーの入れ子状態の管理で、キーボード操作は外側の role="option" の要素で担う */}
            <div
                className="relative flex items-center justify-center overflow-hidden rounded bg-muted text-muted-foreground"
                style={{ width: tileSide, height: tileSide }}
                onMouseEnter={() => setHovering(true)}
                onMouseLeave={() => setHovering(false)}
            >
                {flipbook ? (
                    <div className="h-full w-full" style={flipbook} data-flipbook="" />
                ) : thumbnail.status === "ready" ? (
                    <img
                        src={thumbnail.url}
                        alt=""
                        draggable={false}
                        className={`h-full w-full ${shape === "fit" ? "object-contain" : "object-cover"}`}
                    />
                ) : (
                    <EntryIcon kind={kind} className="h-16 w-16" />
                )}
                {kind === "video" && (
                    <span
                        role="img"
                        aria-label={t("tile.video")}
                        className="absolute bottom-1 right-1 rounded bg-black/60 px-1 text-[10px] leading-4 text-white"
                    >
                        ▶
                    </span>
                )}
            </div>
            <span className="truncate px-1 pt-1 text-center text-xs">{entry.name}</span>
            {place !== undefined && (
                <span
                    title={place}
                    className="truncate px-1 text-center text-xs leading-5 text-muted-foreground"
                    style={{ height: PLACE_HEIGHT }}
                >
                    {place}
                </span>
            )}
        </div>
    );
}
