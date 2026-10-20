import {
    ArrowUpDown,
    Crop,
    Expand,
    Image as ImageIcon,
    LayoutGrid,
    List as ListIcon,
    SlidersHorizontal,
} from "lucide-react";
import { useEffect, useRef } from "react";

import { MAX_TILE_SIZE, MIN_TILE_SIZE } from "../library/grid";
import { type DisplaySet, type SortPreference, usePreferences } from "../library/preferences";
import { directionFromKey, directionFromWheel, isTypingTarget, stepTileSize } from "../library/shortcuts";
import { ORDER_LABELS, SORT_KEYS, SORT_LABELS } from "../library/sort";
import { Button } from "./ui/button";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuLabel,
    DropdownMenuRadioGroup,
    DropdownMenuRadioItem,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from "./ui/dropdown-menu";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { Slider } from "./ui/slider";
import { ToggleGroup, ToggleGroupItem } from "./ui/toggle-group";

/// ホイールで続けて動かしたときに、段階が一気に端まで動かないよう、1 回の増減の間に空ける時間。
const WHEEL_STEP_INTERVAL_MS = 150;

/// 一覧の表示、タイルサイズ、サムネイルの形、並べ替えの操作。今使っている組の値を直接変える。
/// 広い画面でも狭い画面でも、「表示」のポップオーバーにまとめる。
export function ViewControls({ timeline = false }: { timeline?: boolean }) {
    const { set, sort, narrow, updateSet, updateSort } = usePreferences();
    useTileSizeShortcuts(!narrow && (timeline || set.view === "grid"), set.tileSize, (tileSize) =>
        updateSet({ tileSize }),
    );

    return (
        <Popover>
            <PopoverTrigger asChild>
                <Button variant="ghost" size={narrow ? "icon" : "sm"} aria-label="表示" title="表示">
                    <SlidersHorizontal />
                    {!narrow && "表示"}
                </Button>
            </PopoverTrigger>
            <PopoverContent align="end" className="w-72">
                <DisplayControls set={set} sort={sort} timeline={timeline} onSet={updateSet} onSort={updateSort} />
            </PopoverContent>
        </Popover>
    );
}

interface DisplayControlsProps {
    set: DisplaySet;
    sort: SortPreference;
    /// タイムラインか。グリッドだけを新しい順に並べるので、一覧の表示と並べ替えを出さない。
    timeline: boolean;
    onSet: (patch: Partial<DisplaySet>) => void;
    onSort: (patch: Partial<SortPreference>) => void;
}

function DisplayControls({ set, sort, timeline, onSet, onSort }: DisplayControlsProps) {
    return (
        <div className="flex flex-col gap-4">
            {!timeline && (
                <ToggleGroup
                    type="single"
                    variant="outline"
                    size="sm"
                    value={set.view}
                    aria-label="一覧の表示"
                    onValueChange={(value) => (value === "grid" || value === "list") && onSet({ view: value })}
                >
                    <ToggleGroupItem value="grid" aria-label="グリッド" title="グリッド">
                        <LayoutGrid />
                    </ToggleGroupItem>
                    <ToggleGroupItem value="list" aria-label="リスト" title="リスト">
                        <ListIcon />
                    </ToggleGroupItem>
                </ToggleGroup>
            )}
            {(timeline || set.view === "grid") && (
                <div className="flex items-center gap-2 px-2">
                    <ImageIcon className="size-3 shrink-0 text-muted-foreground" />
                    <Slider
                        value={[set.tileSize]}
                        min={MIN_TILE_SIZE}
                        max={MAX_TILE_SIZE}
                        step={1}
                        aria-label="タイルサイズ"
                        onValueChange={([tileSize]) => tileSize !== undefined && onSet({ tileSize })}
                    />
                    <ImageIcon className="size-4 shrink-0 text-muted-foreground" />
                </div>
            )}
            <ToggleGroup
                type="single"
                variant="outline"
                size="sm"
                value={set.thumbnailShape}
                aria-label="サムネイルの形"
                // 選んでいる項目をもう一度押すと値が空になるので、無視する。
                onValueChange={(value) => (value === "crop" || value === "fit") && onSet({ thumbnailShape: value })}
            >
                <ToggleGroupItem value="crop" aria-label="切抜き" title="切抜き">
                    <Crop />
                </ToggleGroupItem>
                <ToggleGroupItem value="fit" aria-label="全体表示" title="全体表示">
                    <Expand />
                </ToggleGroupItem>
            </ToggleGroup>
            {!timeline && <SortMenu sort={sort} onSort={onSort} />}
        </div>
    );
}

function SortMenu({ sort, onSort }: { sort: SortPreference; onSort: (patch: Partial<SortPreference>) => void }) {
    return (
        <DropdownMenu>
            <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" aria-label="並べ替え">
                    <ArrowUpDown />
                    {SORT_LABELS[sort.key]}
                </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end" className="w-48">
                <DropdownMenuLabel>並べ替え</DropdownMenuLabel>
                <DropdownMenuRadioGroup
                    value={sort.key}
                    onValueChange={(key) => onSort({ key: key as SortPreference["key"] })}
                >
                    {SORT_KEYS.map((key) => (
                        <DropdownMenuRadioItem key={key} value={key}>
                            {SORT_LABELS[key]}
                        </DropdownMenuRadioItem>
                    ))}
                </DropdownMenuRadioGroup>
                <DropdownMenuSeparator />
                <DropdownMenuRadioGroup
                    value={sort.order}
                    onValueChange={(order) => onSort({ order: order as SortPreference["order"] })}
                >
                    <DropdownMenuRadioItem value="asc">{ORDER_LABELS.asc}</DropdownMenuRadioItem>
                    <DropdownMenuRadioItem value="desc">{ORDER_LABELS.desc}</DropdownMenuRadioItem>
                </DropdownMenuRadioGroup>
            </DropdownMenuContent>
        </DropdownMenu>
    );
}

/// Ctrl か ⌘ を押しながらのホイールと、`+`、`-` のキーで、タイルサイズを 1 段階ずつ変える。
/// 入力欄やダイアログにフォーカスがあるときは反応しない。
function useTileSizeShortcuts(enabled: boolean, tileSize: number, onChange: (tileSize: number) => void) {
    // 入力のたびに監視を付け直さないよう、最新の値を ref で持つ。
    const latest = useRef({ tileSize, onChange });
    useEffect(() => {
        latest.current = { tileSize, onChange };
    });

    useEffect(() => {
        if (!enabled) {
            return;
        }
        let lastWheel = 0;
        const apply = (direction: 1 | -1) => {
            const next = stepTileSize(latest.current.tileSize, direction);
            if (next !== latest.current.tileSize) {
                latest.current.onChange(next);
            }
        };
        const keyDown = (event: KeyboardEvent) => {
            const direction = directionFromKey(event);
            if (direction === undefined || isTypingTarget(event.target)) {
                return;
            }
            event.preventDefault();
            apply(direction);
        };
        const wheel = (event: WheelEvent) => {
            const direction = directionFromWheel(event);
            if (direction === undefined || isTypingTarget(event.target)) {
                return;
            }
            // ブラウザ全体の拡大縮小にならないよう、既定の動作を止める。
            event.preventDefault();
            if (event.timeStamp - lastWheel >= WHEEL_STEP_INTERVAL_MS) {
                lastWheel = event.timeStamp;
                apply(direction);
            }
        };
        window.addEventListener("keydown", keyDown);
        window.addEventListener("wheel", wheel, { passive: false });
        return () => {
            window.removeEventListener("keydown", keyDown);
            window.removeEventListener("wheel", wheel);
        };
    }, [enabled]);
}
