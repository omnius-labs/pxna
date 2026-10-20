import { describe, expect, it } from "vitest";

import type { Entry } from "../library/entries";
import { FILMSTRIP_LOOKAHEAD, filmstripWanted } from "./filmstrip";

function entry(index: number, contentType = "image/png"): Entry {
    return {
        name: `${index}`,
        path: `/${index}`,
        kind: "file",
        metadata: { size: 1, readonly: false, contentType },
        file: { archive: false },
    };
}

describe("filmstripWanted", () => {
    it("先頭のページの範囲では、先頭のページだけを読む", () => {
        const { pages } = filmstripWanted({ first: 0, last: 22, count: 1000, entryAt: entry });
        expect([...pages]).toEqual([0]);
    });

    it("最後のページの範囲では、最後のページだけを読み、前のページを外す", () => {
        const { pages } = filmstripWanted({ first: 909, last: 931, count: 1000, entryAt: entry });
        expect([...pages]).toEqual([4]);
    });

    it("ページの境界をまたぐ範囲と、先読みの余白がまたぐ範囲では、両側のページを読む", () => {
        expect([...filmstripWanted({ first: 190, last: 205, count: 1000, entryAt: entry }).pages]).toEqual([0, 1]);
        // 先読みの余白の端（first - 10）が 199 と 200 のどちらかで、前のページを読むかが変わる。
        expect([...filmstripWanted({ first: 209, last: 230, count: 1000, entryAt: entry }).pages]).toEqual([0, 1]);
        expect([...filmstripWanted({ first: 210, last: 230, count: 1000, entryAt: entry }).pages]).toEqual([1]);
    });

    it("一覧の端では、一覧の外のページを読まない", () => {
        const { pages } = filmstripWanted({ first: 990, last: 999, count: 1000, entryAt: entry });
        expect([...pages]).toEqual([4]);
        expect([...filmstripWanted({ first: 0, last: 0, count: 0, entryAt: entry }).pages]).toEqual([]);
    });

    it("サムネイルは、見えている範囲と余白の画像だけを、中央に近い順に要求する", () => {
        const { thumbnails } = filmstripWanted({ first: 100, last: 110, count: 1000, entryAt: entry });
        const from = 100 - FILMSTRIP_LOOKAHEAD;
        const to = 110 + FILMSTRIP_LOOKAHEAD;
        expect(thumbnails.size).toBe(to - from + 1);
        expect(thumbnails.has(`/${from - 1}`)).toBe(false);
        expect(thumbnails.has(`/${to + 1}`)).toBe(false);
        expect(thumbnails.get("/105")).toBe(0);
        expect(thumbnails.get("/100")).toBe(5);
        expect(thumbnails.get("/115")).toBeGreaterThan(thumbnails.get("/110") ?? Infinity);
    });

    it("画像でない Entry と未読の位置は、サムネイルを要求しない", () => {
        const entryAt = (index: number) =>
            index === 5 ? entry(index, "video/mp4") : index === 6 ? undefined : entry(index);
        const { thumbnails } = filmstripWanted({ first: 0, last: 10, count: 100, entryAt });
        expect(thumbnails.has("/5")).toBe(false);
        expect(thumbnails.has("/6")).toBe(false);
        expect(thumbnails.has("/4")).toBe(true);
    });
});
