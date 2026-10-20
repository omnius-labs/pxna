import { describe, expect, it } from "vitest";

import { loadRouterAt } from "./testing/router";

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

    it.each(["/", "/setup", "/login"])("matches %s", async (path) => {
        expect((await leafAt(path))?.fullPath).toBe(path);
    });

    it("keeps query values as strings", async () => {
        const router = await loadRouterAt("/browse/photos/a?view=1e3");
        expect(router.state.location.search).toEqual({ view: "1e3" });
    });
});

it("検索モードの値を文字列として検証し、flat は 1 だけを通す", async () => {
    const leaf = await leafAt("/search/photos?q=1e3&path=%2Fa&flat=1&view=a%2Fb.png");
    expect(leaf?.fullPath).toBe("/search/$libraryId");
    expect(leaf?.search).toEqual({ q: "1e3", path: "/a", flat: "1", view: "a/b.png" });
    expect((await leafAt("/search/photos?q=a&flat=true"))?._strictSearch).toEqual({ q: "a" });
});
