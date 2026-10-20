// @vitest-environment jsdom
import { queryOptions } from "@tanstack/react-query";
import { createMemoryHistory } from "@tanstack/react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { ApiError } from "../api/client";
import { createAppRouter } from "../router";
import { createAppQueryClient } from "./unauthorized";

const session = vi.hoisted(() => ({
    authenticated: false,
    initialized: false,
}));

vi.mock("./session", async (importOriginal) => {
    const original = await importOriginal<typeof import("./session")>();
    const now = new Date(0).toISOString();
    return {
        ...original,
        currentUserQuery: queryOptions({
            queryKey: ["auth", "me"],
            queryFn: async () => {
                if (!session.authenticated) {
                    throw new ApiError(401, undefined, undefined);
                }
                return {
                    user: { id: "usr_test", username: "owner", role: "owner" as const, createdAt: now, updatedAt: now },
                    session: {
                        id: "ses_test",
                        userId: "usr_test",
                        status: "active" as const,
                        transport: "cookie" as const,
                        createdAt: now,
                        expiresAt: now,
                    },
                };
            },
            retry: false,
            staleTime: Infinity,
        }),
        fetchSetupInitialized: async () => session.initialized,
    };
});

/// main.tsx と同じく、401 を受けたらログインへ移す QueryClient とルータを組み合わせる。
async function openApp(href: string) {
    const history = createMemoryHistory({ initialEntries: [href] });
    const queryClient = createAppQueryClient(() => ({
        currentHref: () => router.state.location.href,
        currentPathname: () => router.state.location.pathname,
        toLogin: (redirect) => void router.navigate({ to: "/login", search: { redirect }, replace: true }),
    }));
    const router = createAppRouter(queryClient, history, false);
    await router.load();
    return { router, history, queryClient };
}

const target = "/browse/photos/a%20b?view=x.jpg";

describe("session guard", () => {
    beforeEach(() => {
        session.authenticated = false;
        session.initialized = false;
    });

    it("sends a visitor to the initial setup while no owner exists", async () => {
        const { router, history } = await openApp(target);
        expect(router.state.location.pathname).toBe("/setup");
        expect(router.state.location.search).toEqual({ redirect: target });
        expect(history.length).toBe(1);
    });

    it("sends a visitor to the login once the owner exists", async () => {
        session.initialized = true;
        const { router, history } = await openApp(target);
        expect(router.state.location.pathname).toBe("/login");
        expect(router.state.location.search).toEqual({ redirect: target });
        expect(history.length).toBe(1);
    });

    it("lets an authenticated visitor open the page", async () => {
        session.authenticated = true;
        const { router } = await openApp(target);
        expect(router.state.location.href).toBe(target);
    });

    it("moves to the login when another query gets 401 while browsing", async () => {
        session.authenticated = true;
        const { router, history, queryClient } = await openApp(target);

        await queryClient
            .fetchQuery({
                queryKey: ["libraries"],
                queryFn: async () => {
                    throw new ApiError(401, undefined, undefined);
                },
            })
            .catch(() => undefined);
        await router.load();

        expect(router.state.location.pathname).toBe("/login");
        expect(router.state.location.search).toEqual({ redirect: target });
        expect(history.length).toBe(1);
        expect(queryClient.getQueryData(["auth", "me"])).toBeUndefined();
    });
});
