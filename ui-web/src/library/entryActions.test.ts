import { QueryClient } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";
import { ApiError } from "../api/client";
import { TIMELINE_SCOPE } from "../location";
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
    kind: "delete_folders",
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
    post.mockResolvedValue(result({ folder: entry }, 201));
    const client = new QueryClient();
    await createFolder(client, "ops", "/made");
    expect(post.mock.calls[0]![1].body).toEqual({ path: "/made", createParents: false });
    expect(refresh).toHaveBeenCalledWith(client, "ops");
});
it.each([false, true])("同じ親に dstPath を渡して名前を変更 %s", async (folder) => {
    post.mockResolvedValue(result({ entries: [entry] }));
    await renameEntry(new QueryClient(), "ops", { path: entry.path, folder }, "changed.txt");
    expect(post.mock.calls[0]![0]).toBe(`/libraries/{libraryId}/${folder ? "folders" : "files"}/move`);
    expect(post.mock.calls[0]![1].body).toEqual({ srcPaths: [entry.path], dstPath: "/base/changed.txt" });
});
it("混在削除を分け、2 つの Operation を保持し、完了前は更新しない", async () => {
    post.mockResolvedValueOnce(result({ operation: op("files") }, 202)).mockResolvedValueOnce(
        result({ operation: op("folders") }, 202),
    );
    const operations = await deleteEntries(
        new QueryClient(),
        "ops",
        new Map([
            ["/a", { folder: false }],
            ["/dir", { folder: true }],
        ]),
    );
    expect(operations.map((o) => o.id)).toEqual(["files", "folders"]);
    expect(post.mock.calls[1]![1].body).toEqual({ srcPaths: ["/dir"], recursive: true });
    expect(refresh).not.toHaveBeenCalled();
});
it("片方の失敗でも成功 Operation を失わない", async () => {
    post.mockResolvedValueOnce(result({ operation: op("files") }, 202)).mockResolvedValueOnce({
        error: { kind: "Forbidden" },
        response: new Response(null, { status: 403 }),
    });
    const error = await deleteEntries(
        new QueryClient(),
        "ops",
        new Map([
            ["/a", { folder: false }],
            ["/dir", { folder: true }],
        ]),
    ).catch((e) => e);
    expect(error).toBeInstanceOf(DeleteEntriesError);
    expect(error).toBeInstanceOf(ApiError);
    expect(error.status).toBe(403);
    expect(error.operations.map((o: { id: string }) => o.id)).toEqual(["files"]);
});
it("同期削除は一覧を更新し、409 は ApiError を保つ", async () => {
    post.mockResolvedValue(result(undefined, 204));
    expect(await deleteEntries(new QueryClient(), "ops", new Map([["/a", { folder: false }]]))).toEqual([]);
    expect(refresh).toHaveBeenCalledOnce();
    post.mockResolvedValue({ error: { kind: "Conflict" }, response: new Response(null, { status: 409 }) });
    await expect(createFolder(new QueryClient(), "ops", "/a")).rejects.toMatchObject({ status: 409, kind: "Conflict" });
});

it("単一 move の同期結果で両 Library を更新し、copy の Operation は保持する", async () => {
    const { moveEntries, copyEntries } = await import("./entryActions");
    post.mockResolvedValueOnce(result({ entries: [entry] }));
    const client = new QueryClient();
    expect((await moveEntries(client, "source", ["/a"], "dest", "/target", "rename")).entries).toEqual([entry]);
    expect(post.mock.calls[0]![1].body).toEqual({
        srcPaths: ["/a"],
        dstLibraryId: "dest",
        dstFolderPath: "/target",
        conflict: "rename",
    });
    expect(refresh.mock.calls.map((args) => args[1])).toEqual(["source", "dest"]);
    refresh.mockClear();
    post.mockResolvedValueOnce(result({ operation: op("copy") }, 202));
    expect((await copyEntries(client, "source", ["/a"], "dest", "/target", "fail")).operations[0]?.id).toBe("copy");
    expect(refresh).not.toHaveBeenCalled();
});
it("混在転送は種別ごとに分け、片方の失敗でも Operation を保持する", async () => {
    const { transferSelection } = await import("./entryActions");
    post.mockResolvedValueOnce(result({ operation: op("accepted") }, 202)).mockResolvedValueOnce({
        error: { kind: "Forbidden" },
        response: new Response(null, { status: 403 }),
    });
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
    );
    expect(transferred.operations.map((item) => item.id)).toEqual(["accepted"]);
    expect(transferred.failures).toHaveLength(1);
    expect(post.mock.calls.map((args) => args[0])).toEqual([
        "/libraries/{libraryId}/files/copy",
        "/libraries/{libraryId}/folders/copy",
    ]);
});
it("同期 skip の空 entries を成功かつスキップとし、409 は結果の失敗に保持する", async () => {
    const { moveEntries } = await import("./entryActions");
    post.mockResolvedValueOnce(result({ entries: [] }));
    expect((await moveEntries(new QueryClient(), "a", ["/file"], "a", "/dest", "skip")).skippedCount).toBe(1);
    post.mockResolvedValueOnce({ error: { kind: "Conflict" }, response: new Response(null, { status: 409 }) });
    expect((await moveEntries(new QueryClient(), "a", ["/file"], "a", "/dest", "fail")).failures[0]).toMatchObject({
        status: 409,
    });
});

it("混在した転送は他方の完了を待たず受理pathを通知する", async () => {
    const { transferSelection } = await import("./entryActions");
    let rejectFolder!: (value: unknown) => void;
    post.mockResolvedValueOnce(result({ operation: op("accepted") }, 202)).mockImplementationOnce(
        () =>
            new Promise((resolve) => {
                rejectFolder = resolve;
            }),
    );
    const accepted = vi.fn();
    const pending = transferSelection(
        new QueryClient(),
        {
            mode: "cut",
            libraryId: "src",
            entries: [
                { path: "/a", folder: false },
                { path: "/dir", folder: true },
            ],
        },
        { libraryId: "dest", path: "/" },
        "fail",
        accepted,
    );
    await vi.waitFor(() =>
        expect(accepted).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ acceptedPaths: ["/a"] })),
    );
    rejectFolder({ error: { kind: "Forbidden" }, response: new Response(null, { status: 403 }) });
    const transferred = await pending;
    expect(transferred.acceptedPaths).toEqual(["/a"]);
    expect(transferred.operations.map((operation) => operation.id)).toEqual(["accepted"]);
    expect(transferred.failures).toHaveLength(1);
});
it("同期転送の一覧更新が失敗しても受理の記録を失わない", async () => {
    const { moveEntries } = await import("./entryActions");
    post.mockResolvedValue(result({ entries: [entry] }));
    refresh.mockRejectedValue(new Error("refresh failed"));
    const accepted = vi.fn();
    const transferred = await moveEntries(new QueryClient(), "src", ["/a"], "dest", "/", "fail", false, accepted);
    expect(accepted).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ acceptedPaths: ["/a"] }));
    expect(transferred.acceptedPaths).toEqual(["/a"]);
    expect(transferred.failures).toHaveLength(2);
});
it("タイムラインの選択は Library ごとに削除し、一部の Library の失敗でもほかの Library の削除を進める", async () => {
    post.mockResolvedValueOnce(result({ entries: [] })).mockResolvedValueOnce({
        error: { kind: "Forbidden" },
        response: new Response(null, { status: 403 }),
    });
    const error = await deleteEntries(
        new QueryClient(),
        TIMELINE_SCOPE,
        new Map([
            ["/a/x.jpg", { folder: false }],
            ["/b/y.jpg", { folder: false }],
            ["/a/z.jpg", { folder: false }],
        ]),
    ).catch((caught: unknown) => caught);
    expect(post.mock.calls.map((call) => [call[1].params.path.libraryId, call[1].body.srcPaths])).toEqual([
        ["a", ["/x.jpg", "/z.jpg"]],
        ["b", ["/y.jpg"]],
    ]);
    expect(refresh).toHaveBeenCalledWith(expect.anything(), "a");
    expect(refresh).toHaveBeenCalledTimes(1);
    expect(error).toBeInstanceOf(DeleteEntriesError);
    expect((error as DeleteEntriesError).acceptedCount).toBe(1);
});
it("タイムラインの鍵の名前を変えるときは、Library 相対パスで要求する", async () => {
    post.mockResolvedValueOnce(result({ entries: [entry] }));
    await renameEntry(new QueryClient(), TIMELINE_SCOPE, { path: "/a/dir/x.jpg", folder: false }, "y.jpg");
    expect(post.mock.calls[0]![1]).toEqual({
        params: { path: { libraryId: "a" } },
        body: { srcPaths: ["/dir/x.jpg"], dstPath: "/dir/y.jpg" },
    });
    expect(refresh).toHaveBeenCalledWith(expect.anything(), "a");
});
