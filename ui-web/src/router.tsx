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
import { lastFolderLocation } from "./library/folderReturn";
import { folderQuery, librariesQuery } from "./library/queries";
import { browseHref, joinPath, paramList, pathSegments, type SearchParam, searchHref, timelineHref } from "./location";
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
/// 繰り返したキーは、現れた順の配列にする。
function parseSearch(searchStr: string): Record<string, string | string[]> {
    const search: Record<string, string | string[]> = {};
    for (const [key, value] of new URLSearchParams(searchStr)) {
        const previous = search[key];
        search[key] = previous === undefined ? value : [...paramList(previous), value];
    }
    return search;
}

function stringifySearch(search: Record<string, unknown>): string {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(search)) {
        if (typeof value === "string") {
            params.set(key, value);
        } else if (Array.isArray(value)) {
            for (const item of value) if (typeof item === "string") params.append(key, item);
        }
    }
    const searchStr = params.toString();
    return searchStr === "" ? "" : `?${searchStr}`;
}

/// 1 つだけの値。繰り返したキーは、最後の値を使う。
function optionalString(value: unknown): string | undefined {
    if (Array.isArray(value)) return optionalString(value.at(-1));
    return typeof value === "string" ? value : undefined;
}

/// 繰り返してよい値。
function optionalParam(value: unknown): SearchParam {
    if (typeof value === "string") return value;
    if (Array.isArray(value)) {
        const items = value.filter((item): item is string => typeof item === "string");
        return items.length > 0 ? items : undefined;
    }
    return undefined;
}

/// 起点からの相対パスだった、旧来の検索の `view` を、Library を含む鍵（先頭の `/` を除く）へ直す。
function legacySearchView(libraryId: string, start: string, view: string | undefined): string | undefined {
    return view === undefined ? undefined : [libraryId, ...pathSegments(start), ...pathSegments(view)].join("/");
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

/// 最後に開いたフォルダへ移す。保存が無ければ先頭の Library のルートへ移す。
/// Library を 1 つも読めないときは、この画面を出したままツリーの読み込み失敗に任せる。
const homeRoute = createRoute({
    getParentRoute: () => authedRoute,
    path: "/",
    beforeLoad: async ({ context }) => {
        const saved = lastFolderLocation();
        const libraries = await context.queryClient
            .fetchQuery({ ...librariesQuery, retry: false })
            .catch(() => undefined);
        const first = libraries?.[0];
        if (first === undefined) {
            return;
        }
        if (saved === undefined || !libraries?.some((library) => library.id === saved.libraryId)) {
            throw redirect({ href: browseHref({ libraryId: first.id, path: "/" }), replace: true });
        }
        // 消えたフォルダへ移ると読み込み失敗の画面になるので、`/` のときだけ存在を確かめる。
        // 404 だけを Library のルートへ戻し、一時的な失敗では保存した場所へ進む。
        if (saved.path !== "/") {
            try {
                await context.queryClient.fetchQuery({ ...folderQuery(saved.libraryId, saved.path), retry: false });
            } catch (error) {
                if (error instanceof ApiError && error.status === 404) {
                    throw redirect({ href: browseHref({ libraryId: saved.libraryId, path: "/" }), replace: true });
                }
            }
        }
        throw redirect({ href: browseHref(saved), replace: true });
    },
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
        const path = joinPath(pathSegments(params._splat ?? ""));
        throw redirect({
            href: searchHref({
                libraryIds: [params.libraryId],
                path,
                query: q,
                flat: search.deep !== "1",
                tags: [],
                view: legacySearchView(params.libraryId, path, search.view),
            }),
            replace: true,
        });
    },
    component: BrowsePage,
});

type SearchQuery = Partial<Record<"lib" | "path" | "flat" | "q" | "rating" | "tag" | "view", string | string[]>>;

const searchRoute = createRoute({
    getParentRoute: () => authedRoute,
    path: "search",
    validateSearch: (search: Record<string, unknown>): SearchQuery => {
        const result: SearchQuery = {};
        for (const key of ["lib", "path", "flat", "q", "rating", "tag", "view"] as const) {
            const value = key === "flat" ? (search.flat === "1" ? "1" : undefined) : optionalParam(search[key]);
            if (value !== undefined) result[key] = typeof value === "string" ? value : [...value];
        }
        return result;
    },
    component: SearchPage,
});

/// Library ごとだったころの検索の URL。その Library を探す `/search` へ置き換え、起点からの相対パスの `view` は鍵へ直す。
const legacySearchRoute = createRoute({
    getParentRoute: () => authedRoute,
    path: "search/$libraryId",
    validateSearch: (search: Record<string, unknown>) => ({
        q: optionalString(search.q),
        path: optionalString(search.path),
        flat: optionalString(search.flat),
        rating: optionalString(search.rating),
        tag: optionalString(search.tag),
        view: optionalString(search.view),
    }),
    beforeLoad: ({ params, search }) => {
        const path = joinPath(pathSegments(search.path ?? ""));
        const rating = search.rating;
        throw redirect({
            href: searchHref({
                libraryIds: [params.libraryId],
                path,
                query: search.q,
                flat: search.flat === "1",
                rating: rating === "good" || rating === "bad" || rating === "none" ? rating : undefined,
                tags: search.tag?.trim() ? [search.tag.trim()] : [],
                view: legacySearchView(params.libraryId, path, search.view),
            }),
            replace: true,
        });
    },
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
    authedRoute.addChildren([
        homeRoute,
        browseRoute,
        searchRoute,
        legacySearchRoute,
        timelineRoute,
        legacyTimelineRoute,
    ]),
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
