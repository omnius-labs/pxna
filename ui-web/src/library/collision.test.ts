import { afterEach, expect, it, vi } from "vitest";
import { findCollisions, resolveConflict } from "./collision";

const { fetch } = vi.hoisted(() => ({ fetch: vi.fn() }));
vi.mock("./entries", () => ({ fetchFolderPage: fetch }));
afterEach(() => fetch.mockReset());
const destination = { libraryId: "dest", path: "/target" };
const page = (names: string[], totalCount = names.length, snapshot = "one") => ({
    entries: names.map((name) => ({ name })),
    totalCount,
    snapshot,
});
it("50 件まで名前ごとに直下を検索し、完全一致だけを衝突とする", async () => {
    fetch.mockImplementation(async (_id, _path, _offset, options) =>
        page([options.query === "one" ? "one-more" : "two"]),
    );
    expect(await findCollisions(["one", "two"], destination)).toEqual(["two"]);
    expect(fetch.mock.calls.map((args) => args[3])).toEqual([
        { query: "one", recursive: false, includeHidden: true },
        { query: "two", recursive: false, includeHidden: true },
    ]);
});
it("50 件では名前ごと、51 件では全ページを照合する", async () => {
    fetch.mockResolvedValue(page([]));
    await findCollisions(
        Array.from({ length: 50 }, (_, i) => String(i)),
        destination,
    );
    expect(fetch).toHaveBeenCalledTimes(50);
    fetch.mockReset();
    fetch.mockResolvedValueOnce(page(["1"], 2)).mockResolvedValueOnce(page(["50"], 2));
    expect(
        await findCollisions(
            Array.from({ length: 51 }, (_, i) => String(i)),
            destination,
        ),
    ).toEqual(["1", "50"]);
    expect(fetch.mock.calls.map((args) => args[2])).toEqual([0, 1]);
    expect(fetch.mock.calls[0]![3].query).toBeUndefined();
});
it("検索結果の次ページまで照合し、版が変わると先頭からやり直す", async () => {
    fetch
        .mockResolvedValueOnce(page(["other"], 2))
        .mockResolvedValueOnce(page(["a"], 2, "two"))
        .mockResolvedValueOnce(page(["a"], 1, "two"));
    expect(await findCollisions(["a"], destination)).toEqual(["a"]);
    expect(fetch.mock.calls.map((args) => args[2])).toEqual([0, 1, 0]);
});
it("衝突なしは fail、衝突ありは一度だけ確認して中止を返す", async () => {
    const confirm = vi.fn().mockResolvedValue("cancel");
    fetch.mockResolvedValue(page([]));
    expect(await resolveConflict(["a"], destination, confirm)).toBe("fail");
    expect(confirm).not.toHaveBeenCalled();
    fetch.mockResolvedValue(page(["a"]));
    expect(await resolveConflict(["a", "a"], destination, confirm)).toBe("cancel");
    expect(confirm).toHaveBeenCalledExactlyOnceWith(["a"]);
});

it("隠し項目を照合し、入力どうしの同名も宛先が空でも確認する", async () => {
    fetch.mockResolvedValue(page([]));
    expect(await findCollisions([".hidden", ".hidden"], { libraryId: "dest", path: "/" })).toEqual([".hidden"]);
    expect(fetch.mock.calls[0]![3].includeHidden).toBe(true);
});
