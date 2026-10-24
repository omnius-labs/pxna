import type { QueryClient } from "@tanstack/react-query";
import {
    createRootRouteWithContext,
    createRoute,
    createRouter,
    Outlet,
    type RouterHistory,
    redirect,
} from "@tanstack/react-router";

import { ApiError } from "./api/client";
import { currentUserQuery, fetchSetupInitialized } from "./auth/session";
import { AuthedOutlet } from "./components/PreferencesGate";
import { joinPath, parseRatingFilter, pathSegments, searchHref, timelineHref } from "./location";
import { BrowsePage } from "./pages/BrowsePage";
import { HomePage } from "./pages/HomePage";
import { LoginPage } from "./pages/LoginPage";
import { SearchPage } from "./pages/SearchPage";
import { SetupPage } from "./pages/SetupPage";
import { TimelinePage } from "./pages/TimelinePage";

export interface RouterContext {
    queryClient: QueryClient;
}

/// クエリの値はすべて文字列として扱う。既定の解釈は値を JSON として読むので、`1e3` という名前が数値になる。
function parseSearch(searchStr: string): Record<string, string> {
    return Object.fromEntries(new URLSearchParams(searchStr));
}

function stringifySearch(search: Record<string, unknown>): string {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(search)) {
        if (typeof value === "string") {
            params.set(key, value);
        }
    }
    const searchStr = params.toString();
    return searchStr === "" ? "" : `?${searchStr}`;
}

function optionalString(value: unknown): string | undefined {
    return typeof value === "string" ? value : undefined;
}

const rootRoute = createRootRouteWithContext<RouterContext>()({
    component: Outlet,
});

/// セッションがなければ、オーナーが未登録なら初期セットアップ、登録済みならログインへ移し、済んだら元の URL へ戻す。
const authedRoute = createRoute({
    getParentRoute: () => rootRoute,
    id: "authed",
    beforeLoad: async ({ context, location }) => {
        try {
            await context.queryClient.ensureQueryData(currentUserQuery);
        } catch (error) {
            if (!(error instanceof ApiError && error.unauthorized)) {
                throw error;
            }
            const initialized = await fetchSetupInitialized();
            throw redirect({ to: initialized ? "/login" : "/setup", search: { redirect: location.href } });
        }
    },
    component: AuthedOutlet,
});

const homeRoute = createRoute({
    getParentRoute: () => authedRoute,
    path: "/",
    component: HomePage,
});

// Library 相対パスは splat に入る。Library のルートでは splat が空になる。
const browseRoute = createRoute({
    getParentRoute: () => authedRoute,
    path: "browse/$libraryId/$",
    validateSearch: (
        search: Record<string, unknown>,
    ): { q?: string; deep?: string; view?: string; reveal?: string } => {
        const view = optionalString(search.view);
        const q = optionalString(search.q);
        const reveal = optionalString(search.reveal);
        return {
            ...(q === undefined ? {} : { q }),
            ...(search.deep === "1" ? { deep: "1" } : {}),
            ...(view === undefined ? {} : { view }),
            ...(reveal === undefined ? {} : { reveal }),
        };
    },
    // 旧来の検索の URL（`q` と `deep=1`）は、検索モードの対応する起点へ置き換える。
    beforeLoad: ({ params, search }) => {
        const q = search.q?.trim();
        if (!q) return;
        throw redirect({
            href: searchHref({
                libraryId: params.libraryId,
                path: joinPath(pathSegments(params._splat ?? "")),
                query: q,
                flat: search.deep !== "1",
                view: search.view,
            }),
            replace: true,
        });
    },
    component: BrowsePage,
});

const searchRoute = createRoute({
    getParentRoute: () => authedRoute,
    path: "search/$libraryId",
    validateSearch: (
        search: Record<string, unknown>,
    ): { q?: string; path?: string; flat?: string; rating?: string; tag?: string; view?: string } => {
        const q = optionalString(search.q);
        const path = optionalString(search.path);
        const rating = parseRatingFilter(search.rating);
        const tag = optionalString(search.tag);
        const view = optionalString(search.view);
        return {
            ...(q === undefined ? {} : { q }),
            ...(path === undefined ? {} : { path }),
            ...(search.flat === "1" ? { flat: "1" } : {}),
            ...(rating === undefined ? {} : { rating }),
            ...(tag === undefined ? {} : { tag }),
            ...(view === undefined ? {} : { view }),
        };
    },
    component: SearchPage,
});

const timelineSearch = (search: Record<string, unknown>): { month?: string; view?: string } => {
    const month = optionalString(search.month);
    const view = optionalString(search.view);
    return { ...(month === undefined ? {} : { month }), ...(view === undefined ? {} : { view }) };
};

const timelineRoute = createRoute({
    getParentRoute: () => authedRoute,
    path: "timeline",
    validateSearch: timelineSearch,
    component: TimelinePage,
});

/// Library ごとだったころのタイムラインの URL。Library を横断するタイムラインへ置き換える。月は残し、Library 相対パスの `view` は鍵へ直す。
const legacyTimelineRoute = createRoute({
    getParentRoute: () => authedRoute,
    path: "timeline/$libraryId",
    validateSearch: timelineSearch,
    beforeLoad: ({ params, search }) => {
        throw redirect({
            href: timelineHref({
                month: search.month,
                view:
                    search.view === undefined
                        ? undefined
                        : `${params.libraryId}/${pathSegments(search.view).join("/")}`,
            }),
            replace: true,
        });
    },
});

const redirectSearch = (search: Record<string, unknown>): { redirect?: string } => {
    const target = optionalString(search.redirect);
    return target === undefined ? {} : { redirect: target };
};

const setupRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "setup",
    validateSearch: redirectSearch,
    component: SetupPage,
});

const loginRoute = createRoute({
    getParentRoute: () => rootRoute,
    path: "login",
    validateSearch: redirectSearch,
    component: LoginPage,
});

const routeTree = rootRoute.addChildren([
    authedRoute.addChildren([homeRoute, browseRoute, searchRoute, timelineRoute, legacyTimelineRoute]),
    setupRoute,
    loginRoute,
]);

/// `isServer` は、DOM のないテストの環境でもブラウザと同じく redirect で画面を移すために渡す。
export function createAppRouter(queryClient: QueryClient, history?: RouterHistory, isServer?: boolean) {
    return createRouter({ routeTree, history, context: { queryClient }, parseSearch, stringifySearch, isServer });
}

declare module "@tanstack/react-router" {
    interface Register {
        router: ReturnType<typeof createAppRouter>;
    }
}
