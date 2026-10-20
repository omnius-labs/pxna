import { formatBytes } from "../library/format";
import type { SelectionProgress } from "../library/useEntryInteraction";
import { ActivityButton } from "./ActivityPanel";

interface StatusBarProps {
    /// サーバが返した全件の数。まだ分からなければ 0 とする。
    totalCount: number | undefined;
    progress?: SelectionProgress;
    selectedCount: number;
    selectedBytes: number | undefined;
    /// 件数の単位。省略すると「件」で、タイムラインは読み込み済みの件数であることを示す。
    countLabel?: string;
}

/// 件数と、選択中の件数と合計サイズを示す。
export function StatusBar({ totalCount, progress, selectedCount, selectedBytes, countLabel = "件" }: StatusBarProps) {
    return (
        <footer className="flex h-7 shrink-0 items-center gap-3 border-t px-3 text-xs text-muted-foreground">
            <span>
                {totalCount ?? 0} {countLabel}
            </span>
            {progress && (
                <span role="status">
                    選択する範囲を読み込み中… {progress.loaded} / {progress.total} ページ
                </span>
            )}
            {selectedCount > 0 && (
                <span className="text-foreground">
                    {selectedCount} 件を選択（{formatBytes(selectedBytes)}）
                </span>
            )}
            <ActivityButton />
        </footer>
    );
}
