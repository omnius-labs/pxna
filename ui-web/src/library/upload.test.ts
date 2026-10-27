import { afterEach, expect, it, vi } from "vitest";
import {
    droppedInputs,
    ONE_SHOT_LIMIT,
    UPLOAD_CHUNK_SIZE,
    UploadStore,
    uploadFile,
    watchUploadsBeforeUnload,
} from "./upload";

const { post, get, del, binary, listing } = vi.hoisted(() => ({
    post: vi.fn(),
    get: vi.fn(),
    del: vi.fn(),
    binary: vi.fn(),
    listing: vi.fn(),
}));
vi.mock("../api/client", async (original) => ({
    ...(await original<typeof import("../api/client")>()),
    api: { POST: post, GET: get, DELETE: del },
}));
vi.mock("./entries", () => ({ fetchFolderPage: listing }));
afterEach(() => {
    vi.unstubAllGlobals();
    post.mockReset();
    get.mockReset();
    del.mockReset();
    binary.mockReset();
    listing.mockReset();
});
const response = (data: unknown, status = 200) => ({ data, response: new Response(null, { status }) });
const destination = { libraryId: "normal", path: "/uploaded.bin" };
function file(size: number, name = "a.bin") {
    return new File([new Uint8Array(size)], name);
}
function setup() {
    vi.stubGlobal("fetch", binary);
    post.mockResolvedValue(response({ upload: { id: "session", committedSize: 0 } }, 201));
    del.mockResolvedValue(response(undefined, 204));
}
it("32 MiB ちょうどまでワンショット、33 MiB は16 MiBずつとcommittedSizeで送る", async () => {
    setup();
    binary.mockResolvedValue(new Response(null, { status: 201 }));
    await uploadFile(file(ONE_SHOT_LIMIT), destination, "rename", new AbortController().signal, () => {});
    expect(binary).toHaveBeenCalledOnce();
    expect(binary.mock.calls[0]![0]).toContain("conflict=rename");
    expect(post).not.toHaveBeenCalled();
    binary.mockClear();
    binary.mockImplementation(async () => new Response(null, { status: 204 }));
    get.mockResolvedValueOnce(response({ upload: { status: "active", committedSize: UPLOAD_CHUNK_SIZE } }))
        .mockResolvedValueOnce(response({ upload: { status: "active", committedSize: UPLOAD_CHUNK_SIZE * 2 } }))
        .mockResolvedValueOnce(response({ upload: { status: "active", committedSize: ONE_SHOT_LIMIT + 1 } }));
    const progress = vi.fn();
    await uploadFile(file(ONE_SHOT_LIMIT + 1), destination, "fail", new AbortController().signal, progress);
    expect(binary.mock.calls.map((args) => args[0].split("offset=")[1])).toEqual([
        "0",
        String(UPLOAD_CHUNK_SIZE),
        String(ONE_SHOT_LIMIT),
    ]);
    expect(binary.mock.calls.map((args) => args[1].body.size)).toEqual([UPLOAD_CHUNK_SIZE, UPLOAD_CHUNK_SIZE, 1]);
    expect(progress.mock.calls.map((args) => args[0])).toEqual([UPLOAD_CHUNK_SIZE, ONE_SHOT_LIMIT, ONE_SHOT_LIMIT + 1]);
    expect(post.mock.calls[1]![0]).toContain("/commit");
    expect(del).not.toHaveBeenCalled();
});
it("実行中のチャンクを止め、セッションを破棄しcommitしない", async () => {
    setup();
    const controller = new AbortController();
    binary.mockImplementation(async (_url, options) => {
        controller.abort();
        options.signal.throwIfAborted();
    });
    await expect(
        uploadFile(file(ONE_SHOT_LIMIT + 1), destination, "fail", controller.signal, () => {}),
    ).rejects.toBeDefined();
    expect(del).toHaveBeenCalledExactlyOnceWith("/libraries/{libraryId}/uploads/{uploadId}", {
        params: { path: { libraryId: "normal", uploadId: "session" } },
    });
    expect(post).toHaveBeenCalledOnce();
});
it("作成中の取り消しでも応答の識別子を受け取って破棄する", async () => {
    setup();
    const controller = new AbortController();
    post.mockImplementationOnce(async () => {
        controller.abort();
        return response({ upload: { id: "created", committedSize: 0 } }, 201);
    });
    await expect(
        uploadFile(file(ONE_SHOT_LIMIT + 1), destination, "fail", controller.signal, () => {}),
    ).rejects.toBeDefined();
    expect(binary).not.toHaveBeenCalled();
    expect(del.mock.calls[0]![1].params.path.uploadId).toBe("created");
});
it("不正なcommittedSizeは失敗し、次のチャンクと公開を行わず破棄する", async () => {
    setup();
    binary.mockResolvedValue(new Response(null, { status: 204 }));
    get.mockResolvedValue(response({ upload: { status: "active", committedSize: 1 } }));
    await expect(
        uploadFile(file(ONE_SHOT_LIMIT + 1), destination, "fail", new AbortController().signal, () => {}),
    ).rejects.toThrow("resume the upload");
    expect(binary).toHaveBeenCalledOnce();
    expect(del).toHaveBeenCalledOnce();
});
it("同時2件、3件目は待機し、待機の取消と完了ごとの一度の一覧更新", async () => {
    setup();
    const resolvers: (() => void)[] = [];
    binary.mockImplementation(
        () => new Promise<Response>((resolve) => resolvers.push(() => resolve(new Response(null, { status: 201 })))),
    );
    const completed = vi.fn().mockResolvedValue(undefined);
    const store = new UploadStore(completed);
    store.enqueue(
        ["one", "two", "three", "four"].map((name) => ({ path: name, file: file(1, name) })),
        { libraryId: "normal", path: "/" },
        "fail",
    );
    await vi.waitFor(() => expect(binary).toHaveBeenCalledTimes(2));
    expect(store.getSnapshot().map((item) => item.status)).toEqual(["running", "running", "pending", "pending"]);
    store.cancel(store.getSnapshot()[2]!.id);
    expect(store.getSnapshot()[2]?.status).toBe("cancelled");
    resolvers[0]!();
    await vi.waitFor(() => expect(binary).toHaveBeenCalledTimes(3));
    resolvers[1]!();
    resolvers[2]!();
    await vi.waitFor(() => expect(store.getSnapshot().filter((item) => item.status === "succeeded")).toHaveLength(3));
    expect(completed).toHaveBeenCalledTimes(3);
});
it("フォルダを先に作り、renameした親へファイルを送り、空フォルダも作る", async () => {
    setup();
    post.mockRejectedValueOnce(
        new (await import("../api/client")).ApiError(409, "Conflict", "exists"),
    ).mockResolvedValue(response({ folder: {} }, 201));
    binary.mockImplementation(async () => {
        expect(post.mock.calls.some((args) => args[1].body.path === "/batch (1)")).toBe(true);
        return new Response(null, { status: 201 });
    });
    const store = new UploadStore();
    store.enqueue(
        [{ path: "batch/file.txt", file: file(1, "file.txt") }, { path: "empty" }],
        { libraryId: "normal", path: "/" },
        "rename",
    );
    await vi.waitFor(() => expect(store.getSnapshot().every((item) => item.status === "succeeded")).toBe(true));
    expect(binary.mock.calls[0]![0]).toContain("path=%2Fbatch+%281%29%2Ffile.txt");
    expect(post.mock.calls.some((args) => args[1].body.path === "/empty")).toBe(true);
});
it("フォルダskipは配下を送らず、overwriteは既存フォルダへマージする", async () => {
    setup();
    const { ApiError } = await import("../api/client");
    post.mockRejectedValue(new ApiError(409, "Conflict", "exists"));
    const skipped = new UploadStore();
    skipped.enqueue([{ path: "batch/a", file: file(1) }], { libraryId: "normal", path: "/" }, "skip");
    await vi.waitFor(() => expect(skipped.getSnapshot()[0]?.status).toBe("skipped"));
    expect(binary).not.toHaveBeenCalled();
    listing.mockResolvedValue({
        entries: [{ name: "batch", kind: "folder", metadata: { readonly: false } }],
        totalCount: 1,
    });
    binary.mockResolvedValue(new Response(null, { status: 201 }));
    const merged = new UploadStore();
    merged.enqueue([{ path: "batch/a", file: file(1) }], { libraryId: "normal", path: "/" }, "overwrite");
    await vi.waitFor(() => expect(merged.getSnapshot()[0]?.status).toBe("succeeded"));
    expect(binary).toHaveBeenCalledOnce();
});
it("送信中だけbeforeunloadを登録し、完了後には外す", async () => {
    setup();
    let finish!: () => void;
    binary.mockImplementation(
        () =>
            new Promise<Response>((resolve) => {
                finish = () => resolve(new Response(null, { status: 201 }));
            }),
    );
    const store = new UploadStore();
    const target = { addEventListener: vi.fn(), removeEventListener: vi.fn() } as unknown as Window;
    const stop = watchUploadsBeforeUnload(store, target);
    expect(target.addEventListener).not.toHaveBeenCalled();
    store.enqueue([{ path: "a", file: file(1) }], { libraryId: "normal", path: "/" }, "fail");
    await vi.waitFor(() => expect(binary).toHaveBeenCalledOnce());
    expect(target.addEventListener).toHaveBeenCalledOnce();
    const event = new Event("beforeunload", { cancelable: true });
    (target.addEventListener as ReturnType<typeof vi.fn>).mock.calls[0]![1](event);
    expect(event.defaultPrevented).toBe(true);
    finish();
    await vi.waitFor(() => expect(target.removeEventListener).toHaveBeenCalledOnce());
    stop();
});
it("directory reader を空になるまで繰り返し、100件を超える子と空フォルダを収集する", async () => {
    const children = Array.from({ length: 101 }, (_, i) => ({
        name: `f${i}`,
        isFile: true,
        file: (resolve: (file: File) => void) => resolve(file(1, `f${i}`)),
    }));
    const pages = [children.slice(0, 100), children.slice(100), []];
    const entry = {
        name: "folder",
        isFile: false,
        createReader: () => ({ readEntries: (resolve: (entries: unknown[]) => void) => resolve(pages.shift()!) }),
    };
    const inputs = await droppedInputs({
        files: [],
        items: [{ webkitGetAsEntry: () => entry }],
    } as unknown as DataTransfer);
    expect(inputs).toHaveLength(102);
    expect(inputs[0]).toEqual({ path: "folder" });
    expect(inputs.at(-1)?.path).toBe("folder/f100");
});
it("403の失敗理由と送信中の取り消しを保存し、次の待機項目を始める", async () => {
    setup();
    binary.mockImplementation(
        (_url, options) =>
            new Promise<Response>((_resolve, reject) =>
                options.signal.addEventListener("abort", () => reject(options.signal.reason)),
            ),
    );
    const store = new UploadStore();
    store.enqueue(
        ["one", "two", "three"].map((name) => ({ path: name, file: file(1, name) })),
        { libraryId: "normal", path: "/" },
        "fail",
    );
    await vi.waitFor(() => expect(binary).toHaveBeenCalledTimes(2));
    binary.mockResolvedValueOnce(
        new Response(JSON.stringify({ kind: "Forbidden", message: "readonly target" }), { status: 403 }),
    );
    store.cancel(store.getSnapshot()[0]!.id);
    await vi.waitFor(() => expect(store.getSnapshot()[2]?.status).toBe("failed"));
    expect(store.getSnapshot()[0]?.status).toBe("cancelled");
    expect(store.getSnapshot()[2]?.error).toBe("This is not allowed.");
    store.cancelAll();
    await vi.waitFor(() => expect(store.getSnapshot()[1]?.status).toBe("cancelled"));
});
it("ファイルのskipは送らず、フォルダoverwriteの種別衝突はファイルを削除してから作る", async () => {
    setup();
    listing.mockResolvedValue({ entries: [{ name: "a", kind: "file", metadata: { readonly: false } }], totalCount: 1 });
    const store = new UploadStore();
    store.enqueue([{ path: "a", file: file(1) }], { libraryId: "normal", path: "/" }, "skip");
    await vi.waitFor(() => expect(store.getSnapshot()[0]?.status).toBe("skipped"));
    expect(binary).not.toHaveBeenCalled();
    const { ApiError } = await import("../api/client");
    post.mockRejectedValueOnce(new ApiError(409, "Conflict", "exists")).mockResolvedValue(
        response({ folder: {} }, 201),
    );
    listing.mockResolvedValue({
        entries: [{ name: "batch", kind: "file", metadata: { readonly: false } }],
        totalCount: 1,
    });
    store.enqueue([{ path: "batch" }], { libraryId: "normal", path: "/" }, "overwrite");
    await vi.waitFor(() => expect(store.getSnapshot()[1]?.status).toBe("succeeded"));
    expect(post.mock.calls.map((args) => args[0])).toEqual([
        "/libraries/{libraryId}/folders",
        "/libraries/{libraryId}/files/delete",
        "/libraries/{libraryId}/folders",
    ]);
});
