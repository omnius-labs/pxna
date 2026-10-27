// @vitest-environment jsdom
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import type { components } from "../api/schema";
import type { Entry } from "../library/entries";
import { formatBytes, formatModifiedAt } from "../library/format";
import { emptySelection, type Selection, selectionItem, selectionValue } from "../library/selection";
import { testEntry } from "../testing/entries";
import { FakeImageSource } from "../testing/images";
import { InfoPanel, InfoPanelToggle } from "./InfoPanel";

const mocks = vi.hoisted(() => ({ get: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../api/client")>()),
    api: { GET: mocks.get },
}));

const selectionOf = (...entries: Entry[]): Selection => ({
    items: new Map(entries.map((entry) => [entry.path, selectionValue(selectionItem(entry, false))])),
    anchor: undefined,
});

/// 情報パネルは、メディア情報の問い合わせに QueryClient を使う。テストごとに新しいキャッシュを使う。
const withClient = (client: QueryClient, node: ReactNode) => (
    <QueryClientProvider client={client}>{node}</QueryClientProvider>
);

async function render(node: ReactNode) {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const client = new QueryClient();
    await act(async () => root.render(withClient(client, node)));
    return {
        host,
        dispose: async () => {
            await act(async () => root.unmount());
            host.remove();
        },
    };
}

it("何も選んでいないとき、Entry を選ぶと詳細が出ることを案内する", async () => {
    const view = await render(<InfoPanel libraryId="photos" selection={emptySelection} />);
    try {
        expect(view.host.textContent).toBe("Choose an item to see its details.");
    } finally {
        await view.dispose();
    }
});

it("ボタンは今の開閉を aria-pressed で示し、押すと切り替えを呼ぶ", async () => {
    const onToggle = vi.fn();
    const view = await render(<InfoPanelToggle open onToggle={onToggle} />);
    try {
        const button = view.host.querySelector<HTMLButtonElement>("button[aria-label='Info panel']");
        expect(button?.getAttribute("aria-pressed")).toBe("true");
        await act(async () => button?.click());
        expect(onToggle).toHaveBeenCalledOnce();
    } finally {
        await view.dispose();
    }
    const closed = await render(<InfoPanelToggle open={false} onToggle={onToggle} />);
    try {
        expect(closed.host.querySelector("button")?.getAttribute("aria-pressed")).toBe("false");
    } finally {
        await closed.dispose();
    }
});

const photo = testEntry("/trip/photo.png", {
    metadata: { size: 2048, contentType: "image/png", modifiedAt: "2026-10-05T01:02:03Z" },
});
const rowOf = (host: HTMLElement, label: string) =>
    [...host.querySelectorAll("dt")].find((term) => term.textContent === label)?.nextElementSibling?.textContent;

it("ファイルを 1 件選ぶと、名前、種類、サイズ、更新日時、Library 相対パスを、要求なしで出す", async () => {
    mocks.get.mockClear();
    const view = await render(<InfoPanel libraryId="photos" selection={selectionOf(photo)} />);
    try {
        expect(view.host.querySelector("h2")?.textContent).toBe("photo.png");
        expect(rowOf(view.host, "Type")).toBe("Image");
        expect(rowOf(view.host, "Size")).toBe(formatBytes(2048));
        expect(rowOf(view.host, "Modified")).toBe(formatModifiedAt("2026-10-05T01:02:03Z"));
        expect(rowOf(view.host, "Place")).toBe("/trip/photo.png");
        expect(mocks.get).not.toHaveBeenCalled();
    } finally {
        await view.dispose();
    }
});

it("フォルダと archive には、サイズの項目を出さない", async () => {
    const folder = testEntry("/trip/day1", { folder: true });
    const archive: Entry = { ...testEntry("/trip/album.zip"), file: { archive: true } };
    for (const [entry, kind] of [
        [folder, "Folder"],
        [archive, "Archive"],
    ] as const) {
        const view = await render(<InfoPanel libraryId="photos" selection={selectionOf(entry)} />);
        try {
            expect(view.host.querySelector("h2")?.textContent).toBe(entry.name);
            expect(rowOf(view.host, "Type")).toBe(kind);
            expect(rowOf(view.host, "Size")).toBeUndefined();
            expect(rowOf(view.host, "Place")).toBe(entry.path);
        } finally {
            await view.dispose();
        }
    }
});

it("選択が変わると、出す項目も変わる", async () => {
    Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
    const host = document.createElement("div");
    document.body.append(host);
    const root = createRoot(host);
    const client = new QueryClient();
    const show = (selection: Selection) =>
        act(async () => root.render(withClient(client, <InfoPanel libraryId="photos" selection={selection} />)));
    try {
        await show(selectionOf(photo));
        expect(host.querySelector("h2")?.textContent).toBe("photo.png");
        const next = testEntry("/trip/clip.mp4", { metadata: { size: 5, contentType: "video/mp4" } });
        await show(selectionOf(next));
        expect(host.querySelector("h2")?.textContent).toBe("clip.mp4");
        expect(rowOf(host, "Type")).toBe("Video");
        await show(emptySelection);
        expect(host.textContent).toBe("Choose an item to see its details.");
    } finally {
        await act(async () => root.unmount());
        host.remove();
    }
});

it("複数を選ぶと、件数と、ステータスバーと同じ合計のサイズを、要求なしで出す", async () => {
    mocks.get.mockClear();
    const entries = [1000, 2000, 3000].map((size, index) => testEntry(`/trip/${index}.png`, { metadata: { size } }));
    const view = await render(<InfoPanel libraryId="photos" selection={selectionOf(...entries)} />);
    try {
        expect(view.host.querySelector("h2")?.textContent).toBe("3 selected");
        expect(view.host.textContent).toContain(`Total ${formatBytes(6000)}`);
        expect(view.host.querySelector("dl")).toBeNull();
        expect(mocks.get).not.toHaveBeenCalled();
    } finally {
        await view.dispose();
    }
});

describe("大きめのサムネイル", () => {
    let source: FakeImageSource;
    let uninstall: () => void;
    /// 欲しいと言われているサムネイルのパス。
    const wantedPaths = () => source.wanted().map((want) => want.spec.path);
    const elapse = (ms: number) => act(async () => vi.advanceTimersByTimeAsync(ms));
    const entryAt = (index: number) =>
        testEntry(`/trip/${index}.png`, { metadata: { contentType: "image/png", size: 1 } });

    beforeEach(() => {
        Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
        vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
        mocks.get.mockReset();
        source = new FakeImageSource();
        uninstall = source.install();
    });
    afterEach(() => {
        uninstall();
        vi.useRealTimers();
        vi.unstubAllGlobals();
    });

    async function mount(initial: Selection) {
        const host = document.createElement("div");
        document.body.append(host);
        const root = createRoot(host);
        const client = new QueryClient();
        const show = (selection: Selection) =>
            act(async () => root.render(withClient(client, <InfoPanel libraryId="photos" selection={selection} />)));
        await show(initial);
        return {
            host,
            show,
            unmount: async () => {
                await act(async () => root.unmount());
                host.remove();
            },
        };
    }

    it("選んで 199 ms の間は欲しいと言わず、200 ms たつと言い、形は全体表示で一辺は 256 である", async () => {
        const view = await mount(selectionOf(entryAt(1)));
        await elapse(199);
        expect(wantedPaths()).toEqual([]);
        await elapse(1);
        expect(source.wanted()).toHaveLength(1);
        const [want] = source.wanted();
        expect(want?.spec).toEqual({
            libraryId: "photos",
            path: "/trip/1.png",
            kind: "thumbnail",
            width: 256,
            height: 256,
            shape: "fit",
        });
        expect(want?.class).toBe("visible");
        await act(async () => source.ready(want?.spec ?? ({} as never), "blob:thumb"));
        expect(view.host.querySelector("img")?.getAttribute("src")).toBe("blob:thumb");
        await view.unmount();
    });

    it("200 ms より短い間隔で選択を変え続けても、欲しいと言わず、止まってから最後の 1 件だけを欲しいと言う", async () => {
        const view = await mount(selectionOf(entryAt(0)));
        for (const index of [1, 2, 3, 4]) {
            await elapse(150);
            await view.show(selectionOf(entryAt(index)));
        }
        expect(wantedPaths()).toEqual([]);
        await elapse(200);
        expect(wantedPaths()).toEqual(["/trip/4.png"]);
        await view.unmount();
    });

    it("欲しいと言った後に選択を変えると、前のものを欲しい一覧から外す", async () => {
        const view = await mount(selectionOf(entryAt(1)));
        await elapse(200);
        expect(wantedPaths()).toEqual(["/trip/1.png"]);
        await view.show(selectionOf(entryAt(2)));
        expect(wantedPaths()).toEqual([]);
        await elapse(200);
        expect(wantedPaths()).toEqual(["/trip/2.png"]);
        await view.unmount();
    });

    it("フォルダは代表サムネイルを欲しいと言い、サムネイルを出せないファイルと、複数の選択では言わない", async () => {
        const folder = testEntry("/trip/day1", { folder: true });
        const view = await mount(selectionOf(folder));
        await elapse(200);
        expect(wantedPaths()).toEqual(["/trip/day1"]);
        await view.show(selectionOf(testEntry("/trip/note.txt")));
        await elapse(1000);
        await view.show(selectionOf(entryAt(1), entryAt(2)));
        await elapse(1000);
        expect(wantedPaths()).toEqual([]);
        await view.unmount();
    });

    it("閉じる（描かれなくなる）と、200 ms の前なら欲しいと言わず、後なら欲しい一覧から外す", async () => {
        const early = await mount(selectionOf(entryAt(1)));
        await elapse(100);
        await early.unmount();
        await elapse(1000);
        expect(source.calls.filter((call) => call.wants.size > 0)).toHaveLength(0);

        const late = await mount(selectionOf(entryAt(2)));
        await elapse(200);
        expect(wantedPaths()).toEqual(["/trip/2.png"]);
        await late.unmount();
        expect(wantedPaths()).toEqual([]);
    });
});

describe("メディア情報", () => {
    type Info = components["schemas"]["MediaInfo"];
    const mediaRequests = () => mocks.get.mock.calls.filter(([path]) => path === "/libraries/{libraryId}/media-info");
    const elapse = (ms: number) => act(async () => vi.advanceTimersByTimeAsync(ms));
    /// 200 ms 待ち、再描画で有効になった問い合わせの結果を、描画まで進める。act の区切りごとに、React の更新が反映される。
    const load = async () => {
        await elapse(200);
        await elapse(10);
        await elapse(10);
    };
    const image = (name: string) => testEntry(`/trip/${name}`, { metadata: { contentType: "image/png", size: 1 } });
    const imageInfo = (shooting: components["schemas"]["ShootingInfo"], rotation?: number): Info => ({
        kind: "image",
        image: {
            contentType: "image/png",
            width: 64,
            height: 32,
            ...(rotation === undefined ? {} : { rotation }),
            shooting,
        },
    });

    beforeEach(() => {
        Object.assign(globalThis, { IS_REACT_ACT_ENVIRONMENT: true });
        vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] });
        mocks.get.mockReset();
        vi.stubGlobal(
            "URL",
            Object.assign(URL, { createObjectURL: vi.fn(() => "blob:thumb"), revokeObjectURL: vi.fn() }),
        );
    });
    afterEach(() => {
        vi.useRealTimers();
        vi.unstubAllGlobals();
    });

    /// サムネイルは常に成功させ、メディア情報の応答は `respond` で決める。
    const answer = (respond: () => Promise<unknown>) =>
        mocks.get.mockImplementation(async (path: string) =>
            path === "/libraries/{libraryId}/media-info"
                ? respond()
                : { data: new Blob(["x"]), response: new Response() },
        );

    async function open(selection: Selection) {
        const host = document.createElement("div");
        document.body.append(host);
        const root = createRoot(host);
        const client = new QueryClient();
        const show = (next: Selection) =>
            act(async () => root.render(withClient(client, <InfoPanel libraryId="photos" selection={next} />)));
        await show(selection);
        return {
            host,
            show,
            unmount: async () => {
                await act(async () => root.unmount());
                host.remove();
            },
        };
    }

    it("200 ms たつまで要求せず、たつと 1 回要求して、回転を反映した寸法と、持つ撮影情報だけを出す", async () => {
        answer(async () => ({
            data: {
                mediaInfo: imageInfo(
                    {
                        takenAt: "2026-04-05T14:32:10",
                        cameraMake: "Apple",
                        cameraModel: "iPhone 17 Pro",
                        lensModel: "iPhone 17 Pro back camera 6.86mm f/1.78",
                        focalLengthMm: 6.86,
                        fNumber: 1.78,
                        exposureTimeSecs: 0.004,
                        iso: 64,
                    },
                    90,
                ),
            },
            response: new Response(),
        }));
        const view = await open(selectionOf(image("a.png")));
        await elapse(199);
        expect(mediaRequests()).toHaveLength(0);
        await elapse(1);
        await elapse(10);
        expect(mediaRequests()).toHaveLength(1);
        expect(mediaRequests()[0]?.[1].params).toEqual({
            path: { libraryId: "photos" },
            query: { path: "/trip/a.png" },
        });
        expect(rowOf(view.host, "Dimensions")).toBe("32 × 64");
        expect(rowOf(view.host, "Date taken")).toBe("2026-04-05 14:32");
        expect(rowOf(view.host, "Camera")).toBe("Apple iPhone 17 Pro");
        expect(rowOf(view.host, "Lens")).toBe("iPhone 17 Pro back camera 6.86mm f/1.78");
        expect(rowOf(view.host, "Focal length")).toBe("6.9 mm");
        expect(rowOf(view.host, "Aperture")).toBe("f/1.8");
        expect(rowOf(view.host, "Exposure time")).toBe("1/250 s");
        expect(rowOf(view.host, "ISO")).toBe("64");
        await view.unmount();
    });

    it("撮影情報は、応答が持つ項目だけを出し、1 つも持たないときは項目を出さない", async () => {
        answer(async () => ({ data: { mediaInfo: imageInfo({ iso: 100 }) }, response: new Response() }));
        const some = await open(selectionOf(image("a.png")));
        await load();
        expect(rowOf(some.host, "ISO")).toBe("100");
        expect(rowOf(some.host, "Date taken")).toBeUndefined();
        expect(rowOf(some.host, "Dimensions")).toBe("64 × 32");
        await some.unmount();

        answer(async () => ({ data: { mediaInfo: imageInfo({}) }, response: new Response() }));
        const none = await open(selectionOf(image("b.png")));
        await load();
        expect(rowOf(none.host, "Dimensions")).toBe("64 × 32");
        expect(none.host.textContent).not.toContain("Shooting info");
        await none.unmount();
    });

    it("動画は、長さ、解像度、コーデックを出す", async () => {
        answer(async () => ({
            data: {
                mediaInfo: {
                    kind: "video",
                    video: {
                        container: "mov,mp4,m4a",
                        videoCodec: "h264",
                        audioCodec: "aac",
                        durationSecs: 2,
                        width: 320,
                        height: 240,
                        frameRate: 10,
                        contentType: "video/mp4",
                    },
                } satisfies Info,
            },
            response: new Response(),
        }));
        const clip = testEntry("/trip/clip.mp4", { metadata: { contentType: "video/mp4", size: 1 } });
        const view = await open(selectionOf(clip));
        await load();
        expect(rowOf(view.host, "Duration")).toBe("0:02");
        expect(rowOf(view.host, "Resolution")).toBe("320 × 240");
        expect(rowOf(view.host, "Codec")).toBe("h264 / aac");
        await view.unmount();
    });

    it("読み込み中は「読み込み中…」を出し、失敗したときは文で知らせ、名前などの項目を残す", async () => {
        answer(() => new Promise(() => {}));
        const loading = await open(selectionOf(image("a.png")));
        await load();
        expect(loading.host.textContent).toContain("Loading…");
        await loading.unmount();

        answer(async () => {
            throw new Error("boom");
        });
        const failed = await open(selectionOf(image("b.png")));
        await load();
        expect(failed.host.querySelector("[role=alert]")?.textContent).toBe("Could not load the media information.");
        expect(failed.host.querySelector("h2")?.textContent).toBe("b.png");
        expect(rowOf(failed.host, "Place")).toBe("/trip/b.png");
        await failed.unmount();
    });

    it("選択を 200 ms より短い間隔で変え続けても要求せず、送信済みの要求は、選択が変わると取り消す", async () => {
        answer(() => new Promise(() => {}));
        const view = await open(selectionOf(image("0.png")));
        for (const name of ["1.png", "2.png", "3.png"]) {
            await elapse(150);
            await view.show(selectionOf(image(name)));
        }
        expect(mediaRequests()).toHaveLength(0);
        await elapse(200);
        expect(mediaRequests().map(([, options]) => options.params.query.path)).toEqual(["/trip/3.png"]);
        const signal = mediaRequests()[0]?.[1].signal as AbortSignal;
        expect(signal.aborted).toBe(false);
        await view.show(selectionOf(image("4.png")));
        expect(signal.aborted).toBe(true);
        await view.unmount();
    });

    it("画像でも動画でもないファイルと、フォルダと、複数の選択では、要求しない", async () => {
        answer(() => new Promise(() => {}));
        const view = await open(selectionOf(testEntry("/trip/note.txt")));
        await elapse(1000);
        await view.show(selectionOf(testEntry("/trip/day1", { folder: true })));
        await elapse(1000);
        await view.show(selectionOf(image("a.png"), image("b.png")));
        await elapse(1000);
        expect(mediaRequests()).toHaveLength(0);
        await view.unmount();
    });
});
