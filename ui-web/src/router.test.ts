// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest";

import { LAST_FOLDER_KEY } from "./library/folderReturn";
import { loadRouterAt } from "./testing/router";

const mocks = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("./api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("./api/client")>()),
    api: { GET: mocks.get },
}));

const libraries = (...ids: string[]) => ({
    data: { libraries: ids.map((id) => ({ id, name: id, readonly: false })) },
    response: new Response(null, { status: 200 }),
});
const found = () => ({ data: { folder: {} }, response: new Response(null, { status: 200 }) });
const failed = (status: number) => ({ error: { kind: "NotFound" }, response: new Response(null, { status }) });
const saveFolder = (libraryId: string, path: string) =>
    window.localStorage.setItem(LAST_FOLDER_KEY, JSON.stringify({ libraryId, path }));

beforeEach(() => {
    window.localStorage.clear();
    mocks.get.mockReset();
    mocks.get.mockImplementation(async (path: string) =>
        path === "/libraries" ? libraries("photos", "videos") : found(),
    );
});

async function leafAt(href: string) {
    return (await loadRouterAt(href)).state.matches.at(-1);
}

describe("router", () => {
    it("matches the browse route with a nested Library-relative path", async () => {
        const match = await leafAt("/browse/photos/2024/trip");
        expect(match?.fullPath).toBe("/browse/$libraryId/$");
        expect(match?.params).toMatchObject({ libraryId: "photos", _splat: "2024/trip" });
    });

    it("matches the browse route at the Library root", async () => {
        const match = await leafAt("/browse/photos");
        expect(match?.fullPath).toBe("/browse/$libraryId/$");
        expect(match?.params).toMatchObject({ libraryId: "photos" });
    });

    it.each(["/setup", "/login"])("matches %s", async (path) => {
        expect((await leafAt(path))?.fullPath).toBe(path);
    });

    it("keeps query values as strings", async () => {
        const router = await loadRouterAt("/browse/photos/a?view=1e3");
        expect(router.state.location.search).toEqual({ view: "1e3" });
    });
});

describe("/ の復元", () => {
    it("保存が無ければ先頭の Library のルートへ置き換える", async () => {
        const router = await loadRouterAt("/");
        expect(router.state.location.pathname).toBe("/browse/photos");
        expect(router.history.length).toBe(1);
        expect(mocks.get).toHaveBeenCalledWith("/libraries");
    });

    it("保存したフォルダへ置き換える", async () => {
        saveFolder("videos", "/a/b");
        const router = await loadRouterAt("/");
        expect(router.state.location.pathname).toBe("/browse/videos/a/b");
        expect(mocks.get).toHaveBeenCalledWith(
            "/libraries/{libraryId}/folders",
            expect.objectContaining({ params: { path: { libraryId: "videos" }, query: { path: "/a/b" } } }),
        );
    });

    it("保存した Library が無ければ先頭の Library のルートへ移る", async () => {
        saveFolder("gone", "/a");
        const router = await loadRouterAt("/");
        expect(router.state.location.pathname).toBe("/browse/photos");
        expect(mocks.get).not.toHaveBeenCalledWith("/libraries/{libraryId}/folders", expect.anything());
    });

    it("保存したフォルダが 404 なら、その Library のルートへ移る", async () => {
        saveFolder("videos", "/a/b");
        mocks.get.mockImplementation(async (path: string) =>
            path === "/libraries" ? libraries("photos", "videos") : failed(404),
        );
        const router = await loadRouterAt("/");
        expect(router.state.location.pathname).toBe("/browse/videos");
    });

    it("保存したフォルダの確認が 404 以外で失敗したら、保存した場所へ進む", async () => {
        saveFolder("videos", "/a/b");
        mocks.get.mockImplementation(async (path: string) =>
            path === "/libraries" ? libraries("photos", "videos") : failed(500),
        );
        const router = await loadRouterAt("/");
        expect(router.state.location.pathname).toBe("/browse/videos/a/b");
    });

    it("保存したフォルダの確認が通信の失敗で終わったら、保存した場所へ進む", async () => {
        saveFolder("videos", "/a/b");
        mocks.get.mockImplementation(async (path: string) => {
            if (path === "/libraries") return libraries("photos", "videos");
            throw new TypeError("network error");
        });
        const router = await loadRouterAt("/");
        expect(router.state.location.pathname).toBe("/browse/videos/a/b");
    });

    it("Library が 1 つも無ければ、/ の画面のままにする", async () => {
        mocks.get.mockImplementation(async () => libraries());
        const router = await loadRouterAt("/");
        expect(router.state.location.pathname).toBe("/");
    });

    it("Library の一覧を読めなければ、/ の画面のままにする", async () => {
        mocks.get.mockImplementation(async () => failed(500));
        const router = await loadRouterAt("/");
        expect(router.state.location.pathname).toBe("/");
    });

    it("保存した値が壊れていれば、保存が無いものとして先頭の Library へ移る", async () => {
        window.localStorage.setItem(LAST_FOLDER_KEY, "{oops");
        const router = await loadRouterAt("/");
        expect(router.state.location.pathname).toBe("/browse/photos");
    });
});

it("検索モードの値を文字列として検証し、flat は 1 だけを通し、繰り返したキーは配列にする", async () => {
    const leaf = await leafAt("/search?q=1e3&lib=a&lib=b&tag=x&tag=y&flat=1&view=a%2Fb.png");
    expect(leaf?.fullPath).toBe("/search");
    expect(leaf?.search).toEqual({ q: "1e3", lib: ["a", "b"], tag: ["x", "y"], flat: "1", view: "a/b.png" });
    expect((await leafAt("/search?q=a&flat=true"))?._strictSearch).toEqual({ q: "a" });
});
