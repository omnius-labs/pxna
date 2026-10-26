import { describe, expect, it } from "vitest";

import { breadcrumbs, parentLocation } from "./breadcrumb";

describe("breadcrumbs", () => {
    it("starts below the Library and ends with the current folder", () => {
        const crumbs = breadcrumbs({ libraryId: "photos", path: "/2026/京都旅行" });
        expect(crumbs.map((crumb) => crumb.label)).toEqual(["2026", "京都旅行"]);
        expect(crumbs.map((crumb) => crumb.location)).toEqual([
            { libraryId: "photos", path: "/2026" },
            { libraryId: "photos", path: "/2026/京都旅行" },
        ]);
    });

    it("is empty at the root, and drops the viewer's entry", () => {
        expect(breadcrumbs({ libraryId: "photos", path: "/", view: "a.jpg" })).toEqual([]);
        expect(breadcrumbs({ libraryId: "photos", path: "/a", view: "b.jpg" })).toEqual([
            { label: "a", location: { libraryId: "photos", path: "/a" } },
        ]);
    });

    it("walks into an archive like a folder", () => {
        const crumbs = breadcrumbs({ libraryId: "photos", path: "/a.zip/dir" });
        expect(crumbs.map((crumb) => crumb.location.path)).toEqual(["/a.zip", "/a.zip/dir"]);
    });
});

describe("parentLocation", () => {
    it("goes up one folder", () => {
        expect(parentLocation({ libraryId: "photos", path: "/2026/京都旅行", view: "a.jpg" })).toEqual({
            libraryId: "photos",
            path: "/2026",
        });
        expect(parentLocation({ libraryId: "photos", path: "/2026" })).toEqual({ libraryId: "photos", path: "/" });
    });

    it("does not go above the Library root", () => {
        expect(parentLocation({ libraryId: "photos", path: "/" })).toBeUndefined();
    });
});
