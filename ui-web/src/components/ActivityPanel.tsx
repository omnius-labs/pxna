import { t } from "i18next";
import { currentLanguage } from "../i18n";
import { type ActivityItem, useActivity } from "../library/activity";
import { isRunning } from "../library/operations";
import { useNarrowScreen } from "../library/preferences";
import { isUploadRunning } from "../library/upload";

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
            aria-label={t("activity.dismiss")}
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
            aria-label={t("activity.title")}
            className="fixed bottom-9 right-3 z-30 max-h-[60vh] w-80 overflow-auto rounded-lg border bg-background p-3 shadow-lg"
        >
            <div className="flex items-center justify-between">
                <h2 className="font-semibold">{t("activity.title")}</h2>
                <button type="button" onClick={activity.collapse} aria-label={t("activity.collapse")}>
                    −
                </button>
            </div>
            {activity.error && (
                <p role="alert">
                    {t("activity.loadFailed")}
                    <button type="button" onClick={activity.retry}>
                        {t("common.retry")}
                    </button>
                </p>
            )}
            {activity.items.map((item) => {
                if (item.kind === "upload") {
                    const upload = item.upload;
                    return (
                        <article
                            key={upload.id}
                            className={articleClass(activity.isRead(item), isUploadRunning(upload))}
                        >
                            <DismissButton item={item} />
                            <p className="break-all">
                                {upload.name} · {t(`activity.uploadStatuses.${upload.status}`)}
                            </p>
                            {upload.chunked && (
                                <p>
                                    {t("activity.bytes", {
                                        sent: upload.sent.toLocaleString(currentLanguage()),
                                        size: upload.size.toLocaleString(currentLanguage()),
                                    })}
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
                                    aria-label={t("activity.cancelUpload", { name: upload.name })}
                                    onClick={() => activity.cancelUpload?.(upload.id)}
                                >
                                    {t("common.cancel")}
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
                            {t(`activity.kinds.${op.kind}`)} · {t(`activity.statuses.${op.status}`)}
                        </p>
                        <p>
                            {op.processedCount}
                            {op.totalCount == null ? t("activity.itemsSuffix") : ` / ${op.totalCount}`}
                        </p>
                        {op.skippedCount > 0 && <p>{t("activity.skipped", { count: op.skippedCount })}</p>}
                        {op.failedEntries.map((failure, index) => (
                            /* biome-ignore lint/suspicious/noArrayIndexKey: 同一パスが複数回失敗しうるため index で識別子を一意化している */
                            <p key={`${failure.path}:${index}`} className="break-all text-destructive">
                                {failure.path}: {t(`apiErrors.${failure.kind}`)}
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
                                {t("common.cancel")}
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
            {activity.unreadCount > 0
                ? t("activity.titleUnread", { count: activity.unreadCount })
                : t("activity.title")}
        </button>
    );
}
