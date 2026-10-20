import { createContext, useContext } from "react";
import type { Operation } from "./entryActions";
import type { UploadItem } from "./upload";

export type ActivityItem = { kind: "operation"; operation: Operation } | { kind: "upload"; upload: UploadItem };
/// 項目を識別する値。Operation とアップロードの識別子が重ならないよう、種類を前に付ける。
export const activityKey = (item: ActivityItem): string =>
    item.kind === "upload" ? `upload:${item.upload.id}` : `operation:${item.operation.id}`;

export interface ActivityState {
    items: ActivityItem[];
    /// 開いたときに見せた項目か。既読の項目は薄く表示する。
    isRead(item: ActivityItem): boolean;
    /// 未読の項目の数。
    unreadCount: number;
    /// 項目を一覧から消す。進行中の項目は消せない。
    dismiss(item: ActivityItem): void;
    open: boolean;
    error: Error | null;
    show(operations?: Operation[]): void;
    collapse(): void;
    cancel(id: string): Promise<void>;
    cancelUpload?(id: string): void;
    retry(): void;
}
export const ActivityContext = createContext<ActivityState | undefined>(undefined);
export const useActivity = () => useContext(ActivityContext);
