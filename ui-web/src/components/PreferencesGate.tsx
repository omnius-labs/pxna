import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Outlet } from "@tanstack/react-router";
import { t } from "i18next";
import { type ReactNode, useEffect } from "react";
import { toast } from "sonner";

import { ApiError } from "../api/client";
import { DEFAULT_PREFERENCES, preferencesQuery } from "../library/preferences";
import { ActivityProvider } from "./ActivityProvider";
import { ClipboardProvider } from "./ClipboardProvider";
import { Skeleton } from "./ui/skeleton";

/// 認証済みの画面の入口。表示設定を読み終えるまで、一覧を出さずに骨格だけを出す。
/// 読めなかったときは既定値をキャッシュへ入れて通し、閲覧を止めない。
export function PreferencesGate({ children }: { children: ReactNode }) {
    const client = useQueryClient();
    const query = useQuery(preferencesQuery);
    const unauthorized = query.error instanceof ApiError && query.error.unauthorized;

    useEffect(() => {
        if (query.isError && !unauthorized) {
            client.setQueryData(preferencesQuery.queryKey, DEFAULT_PREFERENCES);
            toast.error(t("preferences.loadFailed"));
        }
    }, [query.isError, unauthorized, client]);

    // 401 は、問い合わせの共通の処理がログインへ移す。
    if (query.isPending || unauthorized) {
        return <ShellSkeleton />;
    }
    return children;
}

/// 認証済みの route の内側。表示設定を読み終えてから、子の route を描く。
export function AuthedOutlet() {
    return (
        <PreferencesGate>
            <ClipboardProvider>
                <ActivityProvider>
                    <Outlet />
                </ActivityProvider>
            </ClipboardProvider>
        </PreferencesGate>
    );
}

function ShellSkeleton() {
    return (
        <div
            role="status"
            aria-label={t("preferences.loading")}
            className="grid h-screen grid-cols-[16rem_1fr] grid-rows-[3rem_1fr_2rem] bg-background"
        >
            <Skeleton className="col-span-2 rounded-none" />
            <Skeleton className="rounded-none" />
            <div />
            <Skeleton className="col-span-2 rounded-none" />
        </div>
    );
}
