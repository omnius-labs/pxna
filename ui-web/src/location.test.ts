import { describe, expect, it } from "vitest";

import {
    type BrowseLocation,
    browseHref,
    browseLocation,
    CROSS_LIBRARY_SCOPE,
    entryAddress,
    groupByLibrary,
    parentPath,
    relativePath,
    type SearchLocation,
    safeRedirect,
    scopedKey,
    searchHref,
    searchLocation,
    timelineHref,
    timelineLocation,
    viewKey,
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
        { path: "/", tags: [] },
        {
            libraryIds: ["photos"],
            path: "/a b/日本語",
            query: "#?%/ 日本語",
            flat: true,
            view: "photos/b/#?%.png",
            tags: [],
        },
        { libraryIds: ["my.lib-1_x", "other"], path: "/", query: "1e3", tags: ["1e3", "旅行 2026"] },
    ])("round-trips %j", async (location) => {
        const router = await loadRouterAt(searchHref(location));
        expect(searchLocation(router.state.location.search as never)).toEqual(location);
    });

    it("repeats the Library and the tag, keeps the start only for one Library, and ignores a rating that is not one of the values", async () => {
        const location: SearchLocation = {
            libraryIds: ["p", "q"],
            path: "/",
            rating: "none",
            tags: ["a", "b"],
            flat: true,
        };
        expect(searchHref({ ...location, path: "/ignored" })).toBe(
            "/search?lib=p&lib=q&flat=1&rating=none&tag=a&tag=b",
        );
        expect(searchHref({ libraryIds: ["p"], path: "/a", tags: [] })).toBe("/search?lib=p&path=%2Fa");
        expect(searchHref({ path: "/a", tags: [] })).toBe("/search");
        const router = await loadRouterAt(searchHref(location));
        expect(searchLocation(router.state.location.search as never)).toEqual(location);
        const invalid = await loadRouterAt("/search?rating=great&tag=%20&lib=p&lib=p&path=%2Fa");
        expect(searchLocation(invalid.state.location.search as never)).toEqual({
            libraryIds: ["p"],
            path: "/a",
            tags: [],
        });
        const several = await loadRouterAt("/search?lib=p&lib=q&path=%2Fa");
        expect(searchLocation(several.state.location.search as never).path).toBe("/");
    });

    it("builds a readable URL and trims the query", () => {
        expect(searchHref({ path: "/", query: " alpha ", tags: [] })).toBe("/search?q=alpha");
        expect(searchHref({ libraryIds: ["p"], path: "/a/b", query: "  ", flat: true, tags: [] })).toBe(
            "/search?lib=p&path=%2Fa%2Fb&flat=1",
        );
        expect(searchHref({ path: "/", tags: [] })).toBe("/search");
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
        expect(scopedKey("my.lib", "/a/b.jpg")).toBe("/my.lib/a/b.jpg");
        expect(entryAddress(CROSS_LIBRARY_SCOPE, "/my.lib/a/b.jpg")).toEqual({ libraryId: "my.lib", path: "/a/b.jpg" });
        expect(entryAddress("photos", "/a/b.jpg")).toEqual({ libraryId: "photos", path: "/a/b.jpg" });
        expect(entryAddress(CROSS_LIBRARY_SCOPE, "/root")).toEqual({ libraryId: "root", path: "/" });
        const groups = groupByLibrary(
            CROSS_LIBRARY_SCOPE,
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
        expect(viewKey("a/b.jpg")).toBe("/a/b.jpg");
        expect(parentPath("/a/b.jpg")).toBe("/a");
        expect(parentPath("/b.jpg")).toBe("/");
        expect(parentPath("/books/vol1.zip/inner/1.png")).toBe("/books/vol1.zip/inner");
    });
});
