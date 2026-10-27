import { t } from "i18next";
import {
    ArrowDownWideNarrow,
    ArrowUpNarrowWide,
    Crop,
    Expand,
    Image as ImageIcon,
    LayoutGrid,
    List as ListIcon,
    Settings,
} from "lucide-react";
import { type ReactNode, useEffect, useRef, useState } from "react";

import { type LanguagePreference, readLanguagePreference, setLanguagePreference } from "../i18n";
import { MAX_TILE_SIZE, MIN_TILE_SIZE } from "../library/grid";
import { type DisplaySet, type SortPreference, usePreferences } from "../library/preferences";
import { directionFromKey, directionFromWheel, isTypingTarget, stepTileSize } from "../library/shortcuts";
import { ORDER_LABELS, SORT_KEYS, SORT_LABELS } from "../library/sort";
import { Button } from "./ui/button";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Slider } from "./ui/slider";
import { ToggleGroup, ToggleGroupItem } from "./ui/toggle-group";

/// ホイールで続けて動かしたときに、段階が一気に端まで動かないよう、1 回の増減の間に空ける時間。
const WHEEL_STEP_INTERVAL_MS = 150;

/// 表示設定のポップオーバーを開く歯車のボタン。今使っている組の値と並べ替えを直接変える。
/// 広い画面でも狭い画面でも同じポップオーバーを使い、もう一方の組はその幅の画面から変える。
export function SettingsMenu({ timeline = false }: { timeline?: boolean }) {
    const { set, sort, narrow, updateSet, updateSort } = usePreferences();
    useTileSizeShortcuts(!narrow && (timeline || set.view === "grid"), set.tileSize, (tileSize) =>
        updateSet({ tileSize }),
    );

    return (
        <Popover>
            <PopoverTrigger asChild>
                <Button variant="ghost" size="icon" aria-label={t("common.settings")} title={t("common.settings")}>
                    <Settings />
                </Button>
            </PopoverTrigger>
            <PopoverContent align="end" className="w-80">
                <DisplaySettings set={set} sort={sort} timeline={timeline} onSet={updateSet} onSort={updateSort} />
            </PopoverContent>
        </Popover>
    );
}

interface DisplaySettingsProps {
    set: DisplaySet;
    sort: SortPreference;
    /// タイムラインか。グリッドだけを新しい順に並べるので、一覧の表示と並べ替えを出さない。
    timeline: boolean;
    onSet: (patch: Partial<DisplaySet>) => void;
    onSort: (patch: Partial<SortPreference>) => void;
}

/// 効いている値の選択だけを出す。ビューアだけの設定（フィルムストリップの大きさと元の画像への差し替え）はビューアの設定にある。
function DisplaySettings({ set, sort, timeline, onSet, onSort }: DisplaySettingsProps) {
    return (
        <div className="flex flex-col gap-4">
            {!timeline && (
                <Field label={t("settings.listView")}>
                    <ToggleGroup
                        type="single"
                        variant="outline"
                        size="sm"
                        value={set.view}
                        aria-label={t("settings.listView")}
                        onValueChange={(value) => (value === "grid" || value === "list") && onSet({ view: value })}
                    >
                        <ToggleGroupItem value="grid">
                            <LayoutGrid />
                            {t("settings.grid")}
                        </ToggleGroupItem>
                        <ToggleGroupItem value="list">
                            <ListIcon />
                            {t("settings.list")}
                        </ToggleGroupItem>
                    </ToggleGroup>
                </Field>
            )}
            {(timeline || set.view === "grid") && (
                <Field label={t("settings.tileSize")}>
                    <div className="flex items-center gap-2">
                        <ImageIcon className="size-3 shrink-0 text-muted-foreground" />
                        <Slider
                            value={[set.tileSize]}
                            min={MIN_TILE_SIZE}
                            max={MAX_TILE_SIZE}
                            step={1}
                            aria-label={t("settings.tileSize")}
                            onValueChange={([tileSize]) => tileSize !== undefined && onSet({ tileSize })}
                        />
                        <ImageIcon className="size-4 shrink-0 text-muted-foreground" />
                    </div>
                </Field>
            )}
            <Field label={t("settings.thumbnailShape")}>
                <ToggleGroup
                    type="single"
                    variant="outline"
                    size="sm"
                    value={set.thumbnailShape}
                    aria-label={t("settings.thumbnailShape")}
                    // 選んでいる項目をもう一度押すと値が空になるので、無視する。
                    onValueChange={(value) => (value === "crop" || value === "fit") && onSet({ thumbnailShape: value })}
                >
                    <ToggleGroupItem value="crop">
                        <Crop />
                        {t("settings.crop")}
                    </ToggleGroupItem>
                    <ToggleGroupItem value="fit">
                        <Expand />
                        {t("settings.fit")}
                    </ToggleGroupItem>
                </ToggleGroup>
            </Field>
            {!timeline && (
                <Field label={t("settings.sort")}>
                    <div className="flex items-center gap-2">
                        <Select value={sort.key} onValueChange={(key) => onSort({ key: key as SortPreference["key"] })}>
                            <SelectTrigger size="sm" className="w-28" aria-label={t("settings.sortKey")}>
                                <SelectValue />
                            </SelectTrigger>
                            <SelectContent>
                                {SORT_KEYS.map((key) => (
                                    <SelectItem key={key} value={key}>
                                        {SORT_LABELS[key]}
                                    </SelectItem>
                                ))}
                            </SelectContent>
                        </Select>
                        <ToggleGroup
                            type="single"
                            variant="outline"
                            size="sm"
                            value={sort.order}
                            aria-label={t("settings.sortOrder")}
                            onValueChange={(order) => (order === "asc" || order === "desc") && onSort({ order })}
                        >
                            <ToggleGroupItem value="asc">
                                <ArrowUpNarrowWide />
                                {ORDER_LABELS.asc}
                            </ToggleGroupItem>
                            <ToggleGroupItem value="desc">
                                <ArrowDownWideNarrow />
                                {ORDER_LABELS.desc}
                            </ToggleGroupItem>
                        </ToggleGroup>
                    </div>
                </Field>
            )}
            <LanguageField />
        </div>
    );
}

/// 表示の言語の選択。ブラウザに保存するので、サーバの設定とは別である。
function LanguageField() {
    // 選んだ言語が今の表示と同じなら画面を作り直さないので、選択の表示は自分で持つ。
    const [preference, setPreference] = useState(readLanguagePreference);
    const change = (value: string) => {
        setPreference(value as LanguagePreference);
        setLanguagePreference(value as LanguagePreference);
    };
    return (
        <Field label={t("language.label")}>
            <Select value={preference} onValueChange={change}>
                <SelectTrigger size="sm" className="w-40" aria-label={t("language.label")}>
                    <SelectValue />
                </SelectTrigger>
                <SelectContent>
                    <SelectItem value="auto">{t("language.auto")}</SelectItem>
                    <SelectItem value="en">{t("language.en")}</SelectItem>
                    <SelectItem value="ja">{t("language.ja")}</SelectItem>
                </SelectContent>
            </Select>
        </Field>
    );
}

/// 見出しと、その値の操作。
function Field({ label, children }: { label: string; children: ReactNode }) {
    return (
        <div className="flex flex-col gap-1.5">
            <span className="text-xs font-medium text-muted-foreground">{label}</span>
            {children}
        </div>
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
