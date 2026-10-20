import { QueryClient } from "@tanstack/react-query";
import { createMemoryHistory } from "@tanstack/react-router";

import { currentUserQuery } from "../auth/session";
import { createAppRouter } from "../router";
import { seedPreferences } from "./preferences";

/// ログイン済みの状態で `href` を開いたルータを作る。ガードがセッションと表示設定を取りに行かないよう、現在のユーザと表示設定をキャッシュに入れておく。
export async function loadRouterAt(href: string) {
    const queryClient = new QueryClient();
    const now = new Date(0).toISOString();
    queryClient.setQueryData(currentUserQuery.queryKey, {
        user: { id: "usr_test", username: "owner", role: "owner", createdAt: now, updatedAt: now },
        session: {
            id: "ses_test",
            userId: "usr_test",
            status: "active",
            transport: "cookie",
            createdAt: now,
            expiresAt: now,
        },
    });
    seedPreferences(queryClient);
    const router = createAppRouter(queryClient, createMemoryHistory({ initialEntries: [href] }));
    await router.load();
    return router;
}
