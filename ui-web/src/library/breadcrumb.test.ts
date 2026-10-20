import { describe, expect, it } from "vitest";

import { breadcrumbs, parentLocation } from "./breadcrumb";

describe("breadcrumbs", () => {
    it("starts with the Library and ends with the current folder", () => {
        const crumbs = breadcrumbs({ libraryId: "photos", path: "/2026/京都旅行" }, "Photos");
        expect(crumbs.map((crumb) => crumb.label)).toEqual(["Photos", "2026", "京都旅行"]);
        expect(crumbs.map((crumb) => crumb.location)).toEqual([
            { libraryId: "photos", path: "/" },
            { libraryId: "photos", path: "/2026" },
            { libraryId: "photos", path: "/2026/京都旅行" },
        ]);
    });

    it("has only the Library at the root, and drops the viewer's entry", () => {
        expect(breadcrumbs({ libraryId: "photos", path: "/", view: "a.jpg" }, "Photos")).toEqual([
            { label: "Photos", location: { libraryId: "photos", path: "/" } },
        ]);
    });

    it("walks into an archive like a folder", () => {
        const crumbs = breadcrumbs({ libraryId: "photos", path: "/a.zip/dir" }, "Photos");
        expect(crumbs.map((crumb) => crumb.location.path)).toEqual(["/", "/a.zip", "/a.zip/dir"]);
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
