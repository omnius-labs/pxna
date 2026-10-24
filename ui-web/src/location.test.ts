import { describe, expect, it } from "vitest";

import {
    type BrowseLocation,
    browseHref,
    browseLocation,
    entryAddress,
    groupByLibrary,
    parentPath,
    relativePath,
    type SearchLocation,
    safeRedirect,
    searchBrowseLocation,
    searchHref,
    searchLocation,
    TIMELINE_SCOPE,
    timelineEntryPath,
    timelineHref,
    timelineKey,
    timelineLocation,
} from "./location";
import { loadRouterAt } from "./testing/router";

/// 位置を URL にしてルータで開き、ルータが復号した値から位置へ戻す。
async function roundTrip(location: BrowseLocation): Promise<BrowseLocation> {
    const router = await loadRouterAt(browseHref(location));
    const match = router.state.matches.at(-1);
    const params = match?.params as { libraryId: string; _splat?: string };
    const search = router.state.location.search as { view?: string; reveal?: string };
    return browseLocation(params.libraryId, params._splat, search);
}

describe("location", () => {
    it.each<BrowseLocation>([
        { libraryId: "photos", path: "/" },
        { libraryId: "photos", path: "/2024/trip" },
        { libraryId: "photos", path: "/a b/c d", view: "x y.jpg" },
        { libraryId: "photos", path: "/100%/50% off", view: "100%.png" },
        { libraryId: "photos", path: "/#tag/what?", view: "a#b?c&d=e.jpg" },
        { libraryId: "photos", path: "/写真/旅行", view: "日本語.jpg" },
        { libraryId: "photos", path: "/books/vol1.zip/inner/chapter 1", view: "001.png" },
        { libraryId: "my.lib-1_x", path: "/1e3", view: "1e3" },
        { libraryId: "photos", path: "/+plus+", view: "a+b.jpg" },
    ])("round-trips $libraryId $path $view", async (location) => {
        expect(await roundTrip(location)).toEqual(location);
    });

    it("builds a readable URL", () => {
        expect(browseHref({ libraryId: "photos", path: "/a b/c#d", view: "x.jpg" })).toBe(
            "/browse/photos/a%20b/c%23d?view=x.jpg",
        );
        expect(browseHref({ libraryId: "photos", path: "/" })).toBe("/browse/photos");
    });

    it("accepts only same-origin redirects", () => {
        expect(safeRedirect("/browse/photos/a")).toBe("/browse/photos/a");
        expect(safeRedirect(undefined)).toBe("/");
        expect(safeRedirect("https://evil.example/")).toBe("/");
        expect(safeRedirect("//evil.example/")).toBe("/");
        expect(safeRedirect("/\\evil.example/")).toBe("/");
    });
});

describe("search location", () => {
    it.each<SearchLocation>([
        { libraryId: "photos", path: "/" },
        { libraryId: "photos", path: "/a b/日本語", query: "#?%/ 日本語", flat: true, view: "b/#?%.png" },
        { libraryId: "my.lib-1_x", path: "/1e3", query: "1e3" },
    ])("round-trips $libraryId $path $query", async (location) => {
        const router = await loadRouterAt(searchHref(location));
        const match = router.state.matches.at(-1);
        const params = match?.params as { libraryId: string };
        expect(searchLocation(params.libraryId, router.state.location.search as never)).toEqual(location);
    });

    it("carries the rating and the tag, and ignores a rating that is not one of the values", async () => {
        const location: SearchLocation = { libraryId: "p", path: "/a", rating: "none", tag: "旅行 2026", flat: true };
        expect(searchHref(location)).toBe("/search/p?rating=none&tag=%E6%97%85%E8%A1%8C+2026&path=%2Fa&flat=1");
        const router = await loadRouterAt(searchHref(location));
        expect(searchLocation("p", router.state.location.search as never)).toEqual(location);
        const invalid = await loadRouterAt("/search/p?rating=great&tag=%20");
        expect(searchLocation("p", invalid.state.location.search as never)).toEqual({ libraryId: "p", path: "/" });
    });

    it("lists the origin recursively when only a rating or a tag is given", () => {
        expect(searchBrowseLocation({ libraryId: "p", path: "/a", rating: "good" })).toEqual({
            libraryId: "p",
            path: "/a",
            rating: "good",
            recursive: true,
        });
        expect(searchBrowseLocation({ libraryId: "p", path: "/a", tag: "x", flat: true })).toEqual({
            libraryId: "p",
            path: "/a",
            tag: "x",
            recursive: false,
        });
    });

    it("builds a readable URL and trims the query", () => {
        expect(searchHref({ libraryId: "p", path: "/", query: " alpha " })).toBe("/search/p?q=alpha");
        expect(searchHref({ libraryId: "p", path: "/a/b", query: "  ", flat: true })).toBe(
            "/search/p?path=%2Fa%2Fb&flat=1",
        );
        expect(searchHref({ libraryId: "p", path: "/" })).toBe("/search/p");
    });

    it("searches recursively from the origin unless flat", () => {
        expect(searchBrowseLocation({ libraryId: "p", path: "/a", query: "x", view: "b.png" })).toEqual({
            libraryId: "p",
            path: "/a",
            query: "x",
            recursive: true,
            view: "b.png",
        });
        expect(searchBrowseLocation({ libraryId: "p", path: "/a", query: "x", flat: true }).recursive).toBe(false);
        expect(searchBrowseLocation({ libraryId: "p", path: "/a" })).toEqual({ libraryId: "p", path: "/a" });
    });
});
it.each([
    ["/a", "/a/b/c.jpg", "b/c.jpg"],
    ["/", "/a/b.jpg", "a/b.jpg"],
    ["/a", "/a/b.jpg", "b.jpg"],
])("相対パスを求める %s %s", (base, path, expected) => {
    expect(relativePath(base, path)).toBe(expected);
});

describe("timeline location", () => {
    it("round-trips the month and the opened entry through a URL", async () => {
        for (const location of [{}, { month: "2024-05" }, { month: "2024-05", view: "my.lib-1_x/a b/日本語#?%.jpg" }]) {
            const router = await loadRouterAt(timelineHref(location));
            const search = router.state.location.search as { month?: string; view?: string };
            expect(timelineLocation(search)).toEqual(location);
        }
    });

    it("drops a month that is not YYYY-MM", () => {
        expect(timelineHref({ month: "2024-13" })).toBe("/timeline");
        expect(timelineLocation({ month: "2024-5" })).toEqual({});
        expect(timelineHref({ month: "2024-05", view: "p/a/b.jpg" })).toBe(
            "/timeline?month=2024-05&view=p%2Fa%2Fb.jpg",
        );
    });

    it("splits a timeline key into the Library and the Library path", () => {
        expect(timelineKey("my.lib", "/a/b.jpg")).toBe("/my.lib/a/b.jpg");
        expect(entryAddress(TIMELINE_SCOPE, "/my.lib/a/b.jpg")).toEqual({ libraryId: "my.lib", path: "/a/b.jpg" });
        expect(entryAddress("photos", "/a/b.jpg")).toEqual({ libraryId: "photos", path: "/a/b.jpg" });
        expect(entryAddress(TIMELINE_SCOPE, "/root")).toEqual({ libraryId: "root", path: "/" });
        const groups = groupByLibrary(
            TIMELINE_SCOPE,
            new Map([
                ["/a/1", 1],
                ["/b/2", 2],
                ["/a/3", 3],
            ]),
        );
        expect([...groups].map(([id, group]) => [id, [...group]])).toEqual([
            [
                "a",
                [
                    ["/1", 1],
                    ["/3", 3],
                ],
            ],
            ["b", [["/2", 2]]],
        ]);
    });

    it("maps an opened entry back to a Library path and finds its parent", () => {
        expect(timelineEntryPath("a/b.jpg")).toBe("/a/b.jpg");
        expect(parentPath("/a/b.jpg")).toBe("/a");
        expect(parentPath("/b.jpg")).toBe("/");
        expect(parentPath("/books/vol1.zip/inner/1.png")).toBe("/books/vol1.zip/inner");
    });
});
