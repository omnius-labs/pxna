import { QueryClient } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { CROSS_LIBRARY_SCOPE } from "../location";
import { canMutate, createFolder, DeleteEntriesError, deleteEntries, renameEntry, validateName } from "./entryActions";

const { post, refresh } = vi.hoisted(() => ({ post: vi.fn(), refresh: vi.fn() }));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { POST: post },
}));
vi.mock("./listing", () => ({ refreshLibrary: refresh }));
afterEach(() => {
    post.mockReset();
    refresh.mockReset();
});
const entry = { path: "/base/a.txt", name: "a.txt", kind: "file", metadata: { size: 1, readonly: false } };
const op = (id: string) => ({
    id,
    kind: "delete",
    status: "pending",
    libraryId: "ops",
    processedCount: 0,
    skippedCount: 0,
    failedEntries: [],
    createdAt: "2026-10-07",
});
const result = (data: unknown, status = 200) => ({ data, response: new Response(null, { status }) });
it.each([
    [false, false, true],
    [false, true, false],
    [true, false, false],
    [true, true, false],
])("readonly の論理和で変更を判定 %s %s", (library, metadata, expected) => {
    expect(canMutate({ readonly: library }, { metadata: { readonly: metadata } })).toBe(expected);
    expect(canMutate({ readonly: library }, undefined)).toBe(false);
});
it.each([
    ["", "empty"],
    ["a/b", "slash"],
    [".", "dot"],
    ["..", "dot"],
    ["a.txt", undefined],
])("名前を検証 %s", (name, expected) => expect(validateName(name!)).toBe(expected));
it("直下にフォルダを作り、一覧を更新", async () => {
    post.mockResolvedValue(result({ entry }, 201));
    const client = new QueryClient();
    await createFolder(client, "ops", "/made");
    expect(post.mock.calls[0]![1].body).toEqual({ path: "/made", createParents: false });
    expect(refresh).toHaveBeenCalledWith(client, "ops");
});
const done = (id: string, extra: Record<string, unknown> = {}) => ({
    ...op(id),
    status: "succeeded",
    finishedAt: "2026-10-07",
    ...extra,
});
const failed = (id: string, kind: string) =>
    done(id, { status: "failed", failedEntries: [{ path: "/a", kind, message: "taken" }] });
it.each([false, true])("同じ親の path へ 1 件を移して名前を変更 %s", async (folder) => {
    post.mockResolvedValue(result({ operation: done("rename", { resultEntries: [entry] }) }, 202));
    expect(await renameEntry(new QueryClient(), "ops", { path: entry.path, folder }, "changed.txt")).toEqual(entry);
    expect(post.mock.calls[0]![0]).toBe("/entries/move");
    expect(post.mock.calls[0]![1].body).toEqual({
        sources: [{ libraryId: "ops", path: entry.path, view: folder ? "folder" : "file" }],
        destination: { libraryId: "ops", path: "/base/changed.txt" },
    });
});
it("名前の変更で失敗した Operation は、その失敗の種類で投げる", async () => {
    post.mockResolvedValue(result({ operation: failed("rename", "Conflict") }, 202));
    await expect(
        renameEntry(new QueryClient(), "ops", { path: entry.path, folder: false }, "b.txt"),
    ).rejects.toMatchObject({ kind: "Conflict" });
    expect(refresh).not.toHaveBeenCalled();
});
it("混在削除を 1 つの要求で送り、実行中の Operation を保持し、完了前は更新しない", async () => {
    post.mockResolvedValueOnce(result({ operation: op("mixed") }, 202));
    const operations = await deleteEntries(
        new QueryClient(),
        "ops",
        new Map([
            ["/a", { folder: false }],
            ["/dir", { folder: true }],
        ]),
    );
    expect(operations.map((o) => o.id)).toEqual(["mixed"]);
    expect(post.mock.calls).toHaveLength(1);
    expect(post.mock.calls[0]![0]).toBe("/entries/delete");
    expect(post.mock.calls[0]![1].body).toEqual({
        targets: [
            { libraryId: "ops", path: "/a", view: "file" },
            { libraryId: "ops", path: "/dir", view: "folder" },
        ],
        recursive: true,
    });
    expect(refresh).not.toHaveBeenCalled();
});
it("応答の前に終わった削除は一覧を更新し、失敗した Operation はその失敗とともに返す", async () => {
    post.mockResolvedValue(result({ operation: done("deleted") }, 202));
    expect(await deleteEntries(new QueryClient(), "ops", new Map([["/a", { folder: false }]]))).toEqual([]);
    expect(refresh).toHaveBeenCalledOnce();
    post.mockResolvedValue(result({ operation: failed("deleted", "Forbidden") }, 202));
    const error = await deleteEntries(new QueryClient(), "ops", new Map([["/a", { folder: false }]])).catch((e) => e);
    expect(error).toBeInstanceOf(DeleteEntriesError);
    expect(error.kind).toBe("Forbidden");
    expect(error.operations.map((o: { id: string }) => o.id)).toEqual(["deleted"]);
    post.mockResolvedValue({ error: { code: "Conflict" }, response: new Response(null, { status: 409 }) });
    await expect(createFolder(new QueryClient(), "ops", "/a")).rejects.toMatchObject({ status: 409, kind: "Conflict" });
});
it("混在転送を 1 つの要求で送り、実行中の Operation を保持する", async () => {
    const { transferSelection } = await import("./entryActions");
    post.mockResolvedValueOnce(result({ operation: op("accepted") }, 202));
    const accepted = vi.fn();
    const transferred = await transferSelection(
        new QueryClient(),
        {
            mode: "copy",
            libraryId: "a",
            entries: [
                { path: "/file", folder: false },
                { path: "/folder", folder: true },
            ],
        },
        { libraryId: "b", path: "/target" },
        "overwrite",
        accepted,
    );
    expect(post.mock.calls.map((args) => args[0])).toEqual(["/entries/copy"]);
    expect(post.mock.calls[0]![1].body).toEqual({
        sources: [
            { libraryId: "a", path: "/file", view: "file" },
            { libraryId: "a", path: "/folder", view: "folder" },
        ],
        destination: { libraryId: "b", folderPath: "/target" },
        conflict: "overwrite",
    });
    expect(transferred.operations.map((item) => item.id)).toEqual(["accepted"]);
    expect(transferred.acceptedPaths).toEqual(["/file", "/folder"]);
    expect(accepted).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ acceptedPaths: ["/file", "/folder"] }));
    expect(refresh).not.toHaveBeenCalled();
});
it("応答の前に終わった移動は結果と skip を返し、両 Library を更新する", async () => {
    const { transferSelection } = await import("./entryActions");
    const client = new QueryClient();
    const source = { mode: "cut" as const, libraryId: "source", entries: [{ path: "/a", folder: false }] };
    post.mockResolvedValueOnce(result({ operation: done("move", { resultEntries: [entry] }) }, 202));
    const moved = await transferSelection(client, source, { libraryId: "dest", path: "/target" }, "rename");
    expect(post.mock.calls[0]![0]).toBe("/entries/move");
    expect(moved.entries).toEqual([entry]);
    expect(moved.operations).toEqual([]);
    expect(refresh.mock.calls.map((args) => args[1])).toEqual(["source", "dest"]);
    post.mockResolvedValueOnce(result({ operation: done("move", { resultEntries: [], skippedCount: 1 }) }, 202));
    expect((await transferSelection(client, source, { libraryId: "source", path: "/" }, "skip")).skippedCount).toBe(1);
    post.mockResolvedValueOnce(result({ operation: failed("move", "Conflict") }, 202));
    expect(
        (await transferSelection(client, source, { libraryId: "source", path: "/" }, "fail")).failures[0],
    ).toMatchObject({ kind: "Conflict" });
    post.mockResolvedValueOnce({ error: { code: "Forbidden" }, response: new Response(null, { status: 403 }) });
    const rejected = await transferSelection(client, source, { libraryId: "source", path: "/" }, "fail");
    expect(rejected.failures[0]).toMatchObject({ status: 403 });
    expect(rejected.acceptedPaths).toEqual([]);
});
it("応答の前に終わった転送の一覧更新が失敗しても受理の記録を失わない", async () => {
    const { transferSelection } = await import("./entryActions");
    post.mockResolvedValue(result({ operation: done("move", { resultEntries: [entry] }) }, 202));
    refresh.mockRejectedValue(new Error("refresh failed"));
    const accepted = vi.fn();
    const transferred = await transferSelection(
        new QueryClient(),
        { mode: "cut", libraryId: "src", entries: [{ path: "/a", folder: false }] },
        { libraryId: "dest", path: "/" },
        "fail",
        accepted,
    );
    expect(accepted).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ acceptedPaths: ["/a"] }));
    expect(transferred.acceptedPaths).toEqual(["/a"]);
    expect(transferred.failures).toHaveLength(2);
});
it("タイムラインの選択は Library ごとに削除し、一部の Library の失敗でもほかの Library の削除を進める", async () => {
    post.mockResolvedValueOnce(result({ operation: done("a") }, 202)).mockResolvedValueOnce({
        error: { code: "Forbidden" },
        response: new Response(null, { status: 403 }),
    });
    const error = await deleteEntries(
        new QueryClient(),
        CROSS_LIBRARY_SCOPE,
        new Map([
            ["/a/x.jpg", { folder: false }],
            ["/b/y.jpg", { folder: false }],
            ["/a/z.jpg", { folder: false }],
        ]),
    ).catch((caught: unknown) => caught);
    expect(
        post.mock.calls.map((call) =>
            call[1].body.targets.map((target: { libraryId: string; path: string }) => target),
        ),
    ).toEqual([
        [
            { libraryId: "a", path: "/x.jpg", view: "file" },
            { libraryId: "a", path: "/z.jpg", view: "file" },
        ],
        [{ libraryId: "b", path: "/y.jpg", view: "file" }],
    ]);
    expect(refresh).toHaveBeenCalledWith(expect.anything(), "a");
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(error).toBeInstanceOf(DeleteEntriesError);
    expect((error as DeleteEntriesError).acceptedCount).toBe(1);
});
it("タイムラインの鍵の名前を変えるときは、Library 相対パスで要求する", async () => {
    post.mockResolvedValueOnce(result({ operation: done("rename", { resultEntries: [entry] }) }, 202));
    await renameEntry(new QueryClient(), CROSS_LIBRARY_SCOPE, { path: "/a/dir/x.jpg", folder: false }, "y.jpg");
    expect(post.mock.calls[0]![1]).toEqual({
        body: {
            sources: [{ libraryId: "a", path: "/dir/x.jpg", view: "file" }],
            destination: { libraryId: "a", path: "/dir/y.jpg" },
        },
    });
    expect(refresh).toHaveBeenCalledWith(expect.anything(), "a");
});
