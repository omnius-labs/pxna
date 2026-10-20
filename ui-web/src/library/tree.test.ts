import { describe, expect, it } from "vitest";

import type { Entry } from "./entries";
import { ancestorKeys, flattenTree, treeKey, treeKeyAction } from "./tree";

function folder(path: string, hasChildren?: boolean): Entry {
    const name = path.split("/").at(-1) ?? path;
    return {
        path,
        name,
        kind: "folder",
        metadata: { size: 0, readonly: false },
        ...(hasChildren === undefined ? {} : { folder: { hasChildren } }),
    };
}

function archive(path: string): Entry {
    return {
        path,
        name: path.split("/").at(-1) ?? path,
        kind: "file",
        metadata: { size: 1, readonly: true },
        file: { archive: true },
    };
}

const libraries = [
    { id: "photos", name: "Photos" },
    { id: "videos", name: "Videos" },
];

describe("tree", () => {
    it("lists the folders to expand for a location", () => {
        expect(ancestorKeys("photos", "/2024/trip")).toEqual(["photos:/", "photos:/2024", "photos:/2024/trip"]);
        expect(ancestorKeys("photos", "/")).toEqual(["photos:/"]);
    });

    it("flattens expanded folders in display order", () => {
        const expanded = new Set(["photos:/", "photos:/2024"]);
        const children = new Map([
            ["photos:/", [folder("/2024"), archive("/book.zip"), folder("/empty", false)]],
            ["photos:/2024", [folder("/2024/trip")]],
        ]);
        const nodes = flattenTree(libraries, expanded, children);
        expect(nodes.map((node) => [node.key, node.level, node.expandable, node.expanded])).toEqual([
            ["photos:/", 1, true, true],
            ["photos:/2024", 2, true, true],
            ["photos:/2024/trip", 3, true, false],
            ["photos:/book.zip", 2, true, false],
            ["photos:/empty", 2, false, false],
            ["videos:/", 1, true, false],
        ]);
    });

    it("moves the focus and expands with arrow keys", () => {
        const nodes = flattenTree(
            libraries,
            new Set(["photos:/"]),
            new Map([["photos:/", [folder("/a"), folder("/b")]]]),
        );
        expect(treeKeyAction(nodes, "photos:/", "ArrowDown")).toEqual({ type: "focus", key: "photos:/a" });
        expect(treeKeyAction(nodes, "photos:/a", "ArrowUp")).toEqual({ type: "focus", key: "photos:/" });
        expect(treeKeyAction(nodes, "photos:/a", "ArrowRight")).toEqual({ type: "expand", key: "photos:/a" });
        expect(treeKeyAction(nodes, "photos:/", "ArrowRight")).toEqual({ type: "focus", key: "photos:/a" });
        expect(treeKeyAction(nodes, "photos:/", "ArrowLeft")).toEqual({ type: "collapse", key: "photos:/" });
        expect(treeKeyAction(nodes, "photos:/b", "ArrowLeft")).toEqual({ type: "focus", key: "photos:/" });
        expect(treeKeyAction(nodes, "photos:/b", "Enter")).toEqual({ type: "open", key: "photos:/b" });
        expect(treeKeyAction(nodes, "photos:/", "End")).toEqual({ type: "focus", key: treeKey("videos", "/") });
        expect(treeKeyAction(nodes, "videos:/", "ArrowDown")).toEqual({ type: "none" });
    });
});

it("ツリーの readonly は Library と Entry の論理和", () => {
    const nodes = flattenTree(
        [
            { id: "ro", name: "Ro", readonly: true },
            { id: "ops", name: "Ops", readonly: false },
        ],
        new Set(["ro:/", "ops:/"]),
        new Map([
            ["ro:/", [folder("/dir")]],
            ["ops:/", [archive("/pack.zip"), folder("/dir")]],
        ]),
    );
    expect(nodes.map((node) => node.readonly)).toEqual([true, true, false, true, false]);
});
