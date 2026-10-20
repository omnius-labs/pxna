import { Settings } from "lucide-react";

import { MAX_TILE_SIZE, MIN_TILE_SIZE, NARROW_COLUMNS, TILE_MIN_SIDES } from "../library/grid";
import { type DisplaySet, usePreferences } from "../library/preferences";
import { ORDER_LABELS, SORT_KEYS, SORT_LABELS } from "../library/sort";
import { Badge } from "./ui/badge";
import { Button } from "./ui/button";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle, DialogTrigger } from "./ui/dialog";
import { Label } from "./ui/label";
import { RadioGroup, RadioGroupItem } from "./ui/radio-group";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "./ui/select";
import { Separator } from "./ui/separator";
import { Slider } from "./ui/slider";

/// 表示設定の画面を開くボタンと、その画面。広い画面と狭い画面の 2 組を並べて出し、今使っている組を示す。
/// 並べ替えは、画面の幅によらない 1 つの値として下に出す。
export function SettingsDialog() {
    const { preferences, narrow, updateSetFor, updateSort } = usePreferences();

    return (
        <Dialog>
            <DialogTrigger asChild>
                <Button variant="ghost" size="icon" aria-label="設定" title="設定">
                    <Settings />
                </Button>
            </DialogTrigger>
            <DialogContent className="sm:max-w-2xl">
                <DialogHeader>
                    <DialogTitle>表示設定</DialogTitle>
                    <DialogDescription>どの端末から開いても、同じ幅の画面には同じ設定が使われます。</DialogDescription>
                </DialogHeader>
                <div className="grid gap-6 sm:grid-cols-2">
                    <SetSettings
                        title="広い画面"
                        narrow={false}
                        current={!narrow}
                        set={preferences.wide}
                        onChange={(patch) => updateSetFor(false, patch)}
                    />
                    <SetSettings
                        title="狭い画面"
                        narrow
                        current={narrow}
                        set={preferences.narrow}
                        onChange={(patch) => updateSetFor(true, patch)}
                    />
                </div>
                <Separator />
                <div className="flex items-center gap-4">
                    <Label className="w-24 shrink-0 text-muted-foreground">並べ替え</Label>
                    <Select
                        value={preferences.sort.key}
                        onValueChange={(key) => updateSort({ key: key as typeof preferences.sort.key })}
                    >
                        <SelectTrigger className="w-32" aria-label="並べ替えのキー">
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
                    <Select
                        value={preferences.sort.order}
                        onValueChange={(order) => updateSort({ order: order as typeof preferences.sort.order })}
                    >
                        <SelectTrigger className="w-24" aria-label="並べ替えの向き">
                            <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                            <SelectItem value="asc">{ORDER_LABELS.asc}</SelectItem>
                            <SelectItem value="desc">{ORDER_LABELS.desc}</SelectItem>
                        </SelectContent>
                    </Select>
                </div>
            </DialogContent>
        </Dialog>
    );
}

interface SetSettingsProps {
    title: string;
    narrow: boolean;
    /// 今の画面の幅で使っている組か。
    current: boolean;
    set: DisplaySet;
    onChange: (patch: Partial<DisplaySet>) => void;
}

/// 各組で使う表示の操作だけを出す。元の画像への差し替えは、一覧の表示によらずビューアを開けるので、どちらの一覧の表示でも出す。
function SetSettings({ title, narrow, current, set, onChange }: SetSettingsProps) {
    const labels = narrow ? NARROW_COLUMNS.map((columns) => `${columns} 列`) : TILE_MIN_SIDES.map(String);
    const name = narrow ? "narrow" : "wide";
    return (
        <section aria-label={title} data-current={current || undefined} className="space-y-4">
            <h3 className="flex items-center gap-2 text-sm font-medium">
                {title}
                {current && <Badge variant="secondary">この画面</Badge>}
            </h3>
            <div className="space-y-1.5">
                <Label className="text-muted-foreground">一覧の表示</Label>
                <RadioGroup
                    value={set.view}
                    className="flex gap-4"
                    aria-label={`${title}の一覧の表示`}
                    onValueChange={(value) => (value === "grid" || value === "list") && onChange({ view: value })}
                >
                    <Label className="font-normal">
                        <RadioGroupItem value="grid" id={`${name}-grid`} />
                        グリッド
                    </Label>
                    <Label className="font-normal">
                        <RadioGroupItem value="list" id={`${name}-list`} />
                        リスト
                    </Label>
                </RadioGroup>
            </div>
            {set.view === "grid" && (
                <div className="space-y-1.5">
                    <Label className="text-muted-foreground">タイルサイズ</Label>
                    <Slider
                        value={[set.tileSize]}
                        min={MIN_TILE_SIZE}
                        max={MAX_TILE_SIZE}
                        step={1}
                        aria-label={`${title}のタイルサイズ`}
                        onValueChange={([tileSize]) => tileSize !== undefined && onChange({ tileSize })}
                    />
                    <div className="flex justify-between text-[11px] text-muted-foreground">
                        {labels.map((label) => (
                            <span key={label}>{label}</span>
                        ))}
                    </div>
                </div>
            )}
            <div className="space-y-1.5">
                <Label className="text-muted-foreground">サムネイルの形</Label>
                <RadioGroup
                    value={set.thumbnailShape}
                    className="flex gap-4"
                    aria-label={`${title}のサムネイルの形`}
                    onValueChange={(value) =>
                        (value === "crop" || value === "fit") && onChange({ thumbnailShape: value })
                    }
                >
                    <Label className="font-normal">
                        <RadioGroupItem value="crop" id={`${name}-crop`} />
                        切抜き
                    </Label>
                    <Label className="font-normal">
                        <RadioGroupItem value="fit" id={`${name}-fit`} />
                        全体表示
                    </Label>
                </RadioGroup>
            </div>
            <div className="space-y-1.5">
                <Label className="text-muted-foreground">元の画像への差し替え</Label>
                <RadioGroup
                    value={set.originalImage}
                    className="flex gap-4"
                    aria-label={`${title}の元の画像への差し替え`}
                    onValueChange={(value) =>
                        (value === "auto" || value === "onZoom") && onChange({ originalImage: value })
                    }
                >
                    <Label className="font-normal">
                        <RadioGroupItem value="auto" id={`${name}-original-auto`} />
                        自動
                    </Label>
                    <Label className="font-normal">
                        <RadioGroupItem value="onZoom" id={`${name}-original-on-zoom`} />
                        拡大したときだけ
                    </Label>
                </RadioGroup>
            </div>
        </section>
    );
}
