import { useQueryClient } from "@tanstack/react-query";
import { t } from "i18next";
import { type ReactNode, useState } from "react";
import { toast } from "sonner";
import type { ActivityState } from "../library/activity";
import { ActivityContext, type ActivityItem, activityKey } from "../library/activity";
import type { Operation } from "../library/entryActions";
import { cancelOperation, isRunning, operationsQuery, useOperations } from "../library/operations";
import { isUploadRunning, useUploads } from "../library/upload";
import { TransferProvider } from "./TransferProvider";
import { UploadProvider } from "./UploadProvider";

export function ActivityProvider({ children }: { children: ReactNode }) {
    const client = useQueryClient();
    const query = useOperations();
    const [collapsed, setCollapsed] = useState(false);
    const operations = query.data ?? [];
    const show = (added: Operation[] = []) => {
        if (added.length)
            client.setQueryData(operationsQuery.queryKey, (current = []) => [
                ...current.filter((op) => !added.some((item) => item.id === op.id)),
                ...added,
            ]);
        setCollapsed(false);
    };
    const cancel = async (id: string) => {
        try {
            const operation = await cancelOperation(id);
            client.setQueryData(operationsQuery.queryKey, (current = []) =>
                current.map((item) => (item.id === id ? operation : item)),
            );
        } catch {
            toast.error(t("activity.undoFailed"));
        }
    };
    return (
        <UploadProvider>
            <ActivityContents
                activity={{
                    isRead: () => false,
                    unreadCount: 0,
                    dismiss: () => {},
                    items: operations.map((operation) => ({ kind: "operation", operation })),
                    open: !collapsed,
                    error: query.error,
                    show,
                    collapse: () => setCollapsed(true),
                    cancel,
                    retry: () => {
                        void query.refetch();
                    },
                }}
            >
                {children}
            </ActivityContents>
        </UploadProvider>
    );
}

const running = (item: ActivityItem): boolean =>
    item.kind === "upload" ? isUploadRunning(item.upload) : isRunning(item.operation);

/// アップロードと Operation を 1 つの一覧にまとめ、既読と消去を管理する。どちらもこのページを開いている間だけ保つ。
function ActivityContents({ activity, children }: { activity: ActivityState; children: ReactNode }) {
    const uploads = useUploads();
    const [read, setRead] = useState<ReadonlySet<string>>(new Set());
    const [dismissed, setDismissed] = useState<ReadonlySet<string>>(new Set());
    const items: ActivityState["items"] = [
        ...uploads.items
            .slice()
            .reverse()
            .map((upload) => ({ kind: "upload" as const, upload })),
        ...activity.items,
    ].filter((item) => !dismissed.has(activityKey(item)));
    const isRead = (item: ActivityItem) => read.has(activityKey(item));
    // 項目を足して開くときは足した項目を見せ、足さずに開くとき（ボタンから開くとき）は、今ある項目を既読にする。
    const show = (added?: Operation[]) => {
        activity.show(added);
        if (!added?.length) setRead(new Set(items.map(activityKey)));
    };
    const dismiss = (item: ActivityItem) => {
        if (running(item)) return;
        setDismissed((current) => new Set(current).add(activityKey(item)));
    };
    return (
        <ActivityContext.Provider
            value={{
                ...activity,
                items,
                show,
                isRead,
                unreadCount: items.filter((item) => !isRead(item)).length,
                dismiss,
                open: activity.open && items.length > 0,
                cancelUpload: uploads.store?.cancel,
            }}
        >
            <TransferProvider>{children}</TransferProvider>
        </ActivityContext.Provider>
    );
}
