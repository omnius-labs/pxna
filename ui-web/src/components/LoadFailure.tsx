import { t } from "i18next";
import { ApiError } from "../api/client";

export function LoadFailure({
    error,
    label,
    onRetry,
    compact,
}: {
    error: Error | null;
    label: string;
    onRetry: () => void;
    compact?: boolean;
}) {
    const timedOut = error instanceof ApiError && error.timedOut;
    return (
        <div role="alert" className={compact ? "flex items-center gap-3 border-t px-3 py-2" : "space-y-2 p-4"}>
            <p className="text-destructive">{timedOut ? t("common.extractTimedOut") : label}</p>
            <button type="button" className="rounded border px-3 py-1 hover:bg-muted" onClick={onRetry}>
                {t("common.reload")}
            </button>
        </div>
    );
}
