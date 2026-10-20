import { type ActivityItem, useActivity } from "../library/activity";
import { isRunning } from "../library/operations";
import { useNarrowScreen } from "../library/preferences";
import { isUploadRunning } from "../library/upload";

const kinds = {
    copy_files: "ファイルのコピー",
    move_files: "ファイルの移動",
    delete_files: "ファイルの削除",
    copy_folders: "フォルダのコピー",
    move_folders: "フォルダの移動",
    delete_folders: "フォルダの削除",
};
const statuses = { pending: "待機中", running: "実行中", succeeded: "完了", failed: "失敗", cancelled: "取り消した" };
/// 既読の項目は薄く表示する。進行中の項目は、既読でも薄くしない。
const articleClass = (read: boolean, running: boolean) =>
    `relative mt-3 border-t pt-2 pr-5 text-xs ${read && !running ? "opacity-60" : ""}`;

/// 項目を一覧から消す。進行中の項目には出さない。
function DismissButton({ item }: { item: ActivityItem }) {
    const activity = useActivity();
    const running = item.kind === "upload" ? isUploadRunning(item.upload) : isRunning(item.operation);
    if (!activity || running) return null;
    return (
        <button
            type="button"
            className="absolute right-0 top-2"
            aria-label="この項目を消す"
            onClick={() => activity.dismiss(item)}
        >
            ×
        </button>
    );
}

export function ActivityPanel() {
    const narrow = useNarrowScreen();
    const activity = useActivity();
    if (narrow || !activity?.open) return null;
    return (
        <section
            aria-label="アクティビティ"
            className="fixed bottom-9 right-3 z-30 max-h-[60vh] w-80 overflow-auto rounded-lg border bg-background p-3 shadow-lg"
        >
            <div className="flex items-center justify-between">
                <h2 className="font-semibold">アクティビティ</h2>
                <button type="button" onClick={activity.collapse} aria-label="アクティビティを畳む">
                    −
                </button>
            </div>
            {activity.error && (
                <p role="alert">
                    操作を読めませんでした。
                    <button type="button" onClick={activity.retry}>
                        再試行
                    </button>
                </p>
            )}
            {activity.items.map((item) => {
                if (item.kind === "upload") {
                    const upload = item.upload;
                    const statuses = {
                        pending: "待機中",
                        running: "送信中",
                        succeeded: "完了",
                        failed: "失敗",
                        cancelled: "取り消した",
                        skipped: "スキップ",
                    };
                    return (
                        <article
                            key={upload.id}
                            className={articleClass(activity.isRead(item), isUploadRunning(upload))}
                        >
                            <DismissButton item={item} />
                            <p className="break-all">
                                {upload.name} · {statuses[upload.status]}
                            </p>
                            {upload.chunked && (
                                <p>
                                    {upload.sent.toLocaleString()} / {upload.size.toLocaleString()} バイト
                                </p>
                            )}
                            {upload.status === "failed" && (
                                <p className="break-all text-destructive">
                                    {upload.path}: {upload.error}
                                </p>
                            )}
                            {isUploadRunning(upload) && (
                                <button
                                    type="button"
                                    className="mt-1 underline"
                                    aria-label={`${upload.name} のアップロードを取り消す`}
                                    onClick={() => activity.cancelUpload?.(upload.id)}
                                >
                                    取り消す
                                </button>
                            )}
                        </article>
                    );
                }
                const op = item.operation;
                return (
                    <article key={op.id} className={articleClass(activity.isRead(item), isRunning(op))}>
                        <DismissButton item={item} />
                        <p>
                            {kinds[op.kind]} · {statuses[op.status]}
                        </p>
                        <p>
                            {op.processedCount}
                            {op.totalCount == null ? " 件" : ` / ${op.totalCount}`}
                        </p>
                        {op.skippedCount > 0 && <p>スキップ {op.skippedCount} 件</p>}
                        {op.failedEntries.map((failure, index) => (
                            /* biome-ignore lint/suspicious/noArrayIndexKey: 同一パスが複数回失敗しうるため index で識別子を一意化している */
                            <p key={`${failure.path}:${index}`} className="break-all text-destructive">
                                {failure.path}: {failure.message ?? failure.kind}
                            </p>
                        ))}
                        {isRunning(op) && (
                            <button
                                type="button"
                                className="mt-1 underline"
                                onClick={() => {
                                    void activity.cancel(op.id);
                                }}
                            >
                                取り消す
                            </button>
                        )}
                    </article>
                );
            })}
        </section>
    );
}
export function ActivityButton() {
    const narrow = useNarrowScreen();
    const activity = useActivity();
    if (narrow || !activity?.items.length || activity.open) return null;
    return (
        <button type="button" className="ml-auto" onClick={() => activity.show()}>
            アクティビティ{activity.unreadCount > 0 && `（${activity.unreadCount}）`}
        </button>
    );
}
