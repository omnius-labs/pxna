import { QueryCache, QueryClient } from "@tanstack/react-query";

import { ApiError } from "../api/client";

/// `QueryClient` が 401 を受けたときに、ログインへ移すために必要なルータの操作。
export interface UnauthorizedNavigator {
    currentHref(): string;
    currentPathname(): string;
    toLogin(redirect: string): void;
}

/// 認証のクエリ（現在のユーザ）の 401 は、route のガードが初期セットアップとログインに振り分けるので、ここでは扱わない。
function isAuthQuery(queryKey: readonly unknown[]): boolean {
    return queryKey[0] === "auth";
}

/// 表示中にセッションが失効して 401 を受けたら、手元のキャッシュを捨ててログインへ移し、ログインの後に今の画面へ戻す。
/// 401 と期限切れ（504）は繰り返しても結果が変わらないので、自動では再試行しない。
export function createAppQueryClient(navigator: () => UnauthorizedNavigator): QueryClient {
    const queryClient: QueryClient = new QueryClient({
        queryCache: new QueryCache({
            onError: (error, query) => {
                if (!(error instanceof ApiError && error.unauthorized) || isAuthQuery(query.queryKey)) {
                    return;
                }
                const nav = navigator();
                if (nav.currentPathname() === "/login" || nav.currentPathname() === "/setup") {
                    return;
                }
                const redirect = nav.currentHref();
                queryClient.clear();
                nav.toLogin(redirect);
            },
        }),
        defaultOptions: {
            queries: {
                retry: (failureCount, error) =>
                    !(error instanceof ApiError && (error.unauthorized || error.timedOut)) && failureCount < 2,
            },
        },
    });
    return queryClient;
}
