import { clientTimings } from "../api/clientTimings";
import type { ClientMessage, FrameHeader, WantedItem } from "../api/imageChannel";
import { cacheGroupOf, createBrowserImageCache, type ImageCache } from "./cache";
import { CLOSE_BROKE_CONTRACT, CLOSE_SESSION_ENDED, decodeFrame, parseServerMessage } from "./protocol";
import type { ImageSource } from "./source";
import {
    compareWants,
    type ErrorKind,
    IDLE,
    type ImageClass,
    type ImageSpec,
    type ImageState,
    specKey,
    type Want,
} from "./spec";

/// `WebSocket` のうち、使う部分。テストが偽物を渡せるようにする。
export type ChannelSocket = Pick<
    WebSocket,
    "binaryType" | "onopen" | "onmessage" | "onclose" | "onerror" | "send" | "close"
>;

/// 1 枚の画像を受け取り終えた（または手元の画像が最新と分かった）ときの記録。
export interface SettledImage {
    spec: ImageSpec;
    /// 欲しいと言ってから受け取り終えるまでの時間（ミリ秒）。
    durationMs: number;
    bytes: number;
    /// 手元の画像がそのまま使えた（`unchanged`）か。
    cached: boolean;
}

export interface ChannelEnvironment {
    open(): ChannelSocket;
    cache: ImageCache;
    createUrl(blob: Blob): string;
    revokeUrl(url: string): void;
    now(): number;
    random(): number;
    setTimeout(callback: () => void, ms: number): unknown;
    clearTimeout(handle: unknown): void;
    /// セッションが終わったと言われたときに呼ぶ。
    onSessionEnded(): void;
    onSettled?(settled: SettledImage): void;
}

/// 再接続を待つ時間は、1 秒から始めて倍にし、30 秒までにする（Image Channel Design §3）。
const BACKOFF_FIRST_MS = 1000;
const BACKOFF_MAX_MS = 30_000;
/// 1 回の want に入れる件数。1 回の text message の上限（256 KiB）に収める。
const MAX_BATCH = 200;
/// 失敗した状態を残す件数の上限。欲しいと言われなくなっても、部品が生きている間は残すので、増え続けないようにする。
const MAX_FAILED = 2000;

type Phase = "looking" | "pending" | "sent" | "done";

interface Item {
    key: string;
    spec: ImageSpec;
    phase: Phase;
    /// 今の接続での名前。送っていなければ持たない。
    id: number | undefined;
    class: ImageClass;
    order: number;
    /// 送った時点の急ぎ度。変わったら送り直す。
    sent: Pick<Want, "class" | "order"> | undefined;
    state: ImageState;
    /// 手元の画像の `ETag`。欲しいと言うときに添える。
    etag: string | null;
    startedAt: number;
    /// この項目を欲しいと言ったことのある部品。失敗した状態は、そのすべてが手放すまで残す。
    touchedBy: Set<object>;
}

interface Assembly {
    header: FrameHeader;
    parts: Uint8Array[];
    received: number;
}

/// image channel の WebSocket 1 本を持ち、部品の「欲しい」を項目にまとめ、受け取った画像を IndexedDB に残す。
///
/// 同じ画像を欲しい複数の部品は 1 つの項目にまとめ、最も急ぐものの急ぎ度で daemon に伝える。
/// 手元に画像があれば先に出し、`ETag` を添えて欲しいと言い、最新なら `unchanged` だけが返る。
export class ImageChannelClient implements ImageSource {
    readonly #env: ChannelEnvironment;
    readonly #items = new Map<string, Item>();
    readonly #byId = new Map<number, Item>();
    readonly #owners = new Map<object, Map<string, Want>>();
    readonly #listeners = new Set<() => void>();
    readonly #assembling = new Map<number, Assembly>();
    #socket: ChannelSocket | undefined;
    #open = false;
    #nextId = 1;
    #backoff = BACKOFF_FIRST_MS;
    #reconnect: unknown;
    #flushScheduled = false;
    #drops: number[] = [];
    /// 契約を破ったと言われた後は、繰り返しても同じなので、つながない。
    #broken = false;

    constructor(env: ChannelEnvironment) {
        this.#env = env;
    }

    getState = (spec: ImageSpec): ImageState => this.#items.get(specKey(spec))?.state ?? IDLE;

    subscribe = (listener: () => void): (() => void) => {
        this.#listeners.add(listener);
        return () => this.#listeners.delete(listener);
    };

    setWants(owner: object, wants: ReadonlyMap<string, Want>): void {
        const previous = this.#owners.get(owner);
        if (wants.size === 0) {
            this.#owners.delete(owner);
        } else {
            this.#owners.set(owner, new Map(wants));
        }
        const keys = new Set<string>([...(previous?.keys() ?? []), ...wants.keys()]);
        for (const key of keys) {
            const spec = wants.get(key)?.spec ?? previous?.get(key)?.spec;
            if (spec !== undefined) this.#reconcile(key, spec, owner, wants.has(key));
        }
        this.#scheduleFlush();
    }

    release(owner: object): void {
        const previous = this.#owners.get(owner);
        this.#owners.delete(owner);
        for (const [key, want] of previous ?? []) {
            this.#reconcile(key, want.spec, owner, false);
        }
        for (const item of [...this.#items.values()]) {
            item.touchedBy.delete(owner);
            if (item.state.status === "failed" && item.touchedBy.size === 0 && this.#best(item.key) === undefined) {
                this.#remove(item);
            }
        }
        this.#scheduleFlush();
    }

    async clear(): Promise<void> {
        this.#env.clearTimeout(this.#reconnect);
        this.#reconnect = undefined;
        const socket = this.#socket;
        this.#detach();
        socket?.close(1000, "cleared");
        for (const item of this.#items.values()) this.#revoke(item);
        this.#items.clear();
        this.#byId.clear();
        this.#owners.clear();
        this.#assembling.clear();
        this.#drops = [];
        this.#backoff = BACKOFF_FIRST_MS;
        this.#broken = false;
        this.#emit();
        await this.#env.cache.clear();
    }

    /// 項目の数。テストが、解放されたことを確かめるために使う。
    get itemCount(): number {
        return this.#items.size;
    }

    #best(key: string): Want | undefined {
        let best: Want | undefined;
        for (const wants of this.#owners.values()) {
            const want = wants.get(key);
            if (want !== undefined && (best === undefined || compareWants(want, best) < 0)) best = want;
        }
        return best;
    }

    #reconcile(key: string, spec: ImageSpec, owner: object, wanted: boolean): void {
        const best = this.#best(key);
        let item = this.#items.get(key);
        if (wanted && item !== undefined) item.touchedBy.add(owner);
        if (best === undefined) {
            // 失敗した状態は、欲しいと言った部品が手放すまで残し、同じ失敗を繰り返さない。
            if (item !== undefined && item.state.status !== "failed") this.#remove(item);
            return;
        }
        if (item === undefined) {
            item = this.#create(key, spec);
            item.touchedBy.add(owner);
        }
        item.class = best.class;
        item.order = best.order;
    }

    #create(key: string, spec: ImageSpec): Item {
        const item: Item = {
            key,
            spec,
            phase: "looking",
            id: undefined,
            class: "prefetch",
            order: 0,
            sent: undefined,
            state: { status: "loading" },
            etag: null,
            startedAt: this.#env.now(),
            touchedBy: new Set(),
        };
        this.#items.set(key, item);
        this.#emit();
        void this.#lookup(item);
        return item;
    }

    /// 手元に画像があれば、先に出す。見つかっても見つからなくても、daemon に欲しいと言う。
    async #lookup(item: Item): Promise<void> {
        const cached = await this.#env.cache.get(item.key).catch(() => undefined);
        if (this.#items.get(item.key) !== item || item.phase !== "looking") return;
        if (cached !== undefined) {
            item.state = {
                status: "ready",
                url: this.#env.createUrl(new Blob([cached.data], { type: cached.contentType })),
                frameCount: cached.frameCount,
            };
            item.etag = cached.etag;
            this.#emit();
        }
        item.phase = "pending";
        this.#scheduleFlush();
    }

    #remove(item: Item): void {
        if (item.phase === "sent" && item.id !== undefined) this.#drops.push(item.id);
        if (item.id !== undefined) {
            this.#byId.delete(item.id);
            this.#assembling.delete(item.id);
        }
        this.#revoke(item);
        this.#items.delete(item.key);
        this.#emit();
    }

    #revoke(item: Item): void {
        if (item.state.status === "ready") this.#env.revokeUrl(item.state.url);
    }

    #scheduleFlush(): void {
        if (this.#flushScheduled) return;
        this.#flushScheduled = true;
        queueMicrotask(() => {
            this.#flushScheduled = false;
            this.#flush();
        });
    }

    /// 送っていない項目と、急ぎ度が変わった項目を、まとめて daemon に伝える。
    #flush(): void {
        const wanting = [...this.#items.values()].filter(
            (item) => item.phase === "pending" || (item.phase === "sent" && this.#changed(item)),
        );
        if (this.#broken) {
            for (const item of wanting) this.#fail(item, "InvalidRequest");
            return;
        }
        if (wanting.length === 0 && this.#drops.length === 0) return;
        if (this.#socket === undefined) {
            // つなぎ直しを待っている間は、つながない。
            if (wanting.length > 0 && this.#reconnect === undefined) this.#connect();
            return;
        }
        if (!this.#open) return;

        if (this.#drops.length > 0) {
            for (let from = 0; from < this.#drops.length; from += MAX_BATCH) {
                this.#send({ type: "drop", ids: this.#drops.slice(from, from + MAX_BATCH) });
            }
            this.#drops = [];
        }
        for (let from = 0; from < wanting.length; from += MAX_BATCH) {
            const items = wanting.slice(from, from + MAX_BATCH).map((item) => this.#wantedItem(item));
            this.#send({ type: "want", items });
        }
    }

    #changed(item: Item): boolean {
        return item.sent === undefined || item.sent.class !== item.class || item.sent.order !== item.order;
    }

    #wantedItem(item: Item): WantedItem {
        if (item.id === undefined) {
            item.id = this.#nextId++;
            this.#byId.set(item.id, item);
        }
        item.phase = "sent";
        item.sent = { class: item.class, order: item.order };
        const { spec } = item;
        return {
            id: item.id,
            libraryId: spec.libraryId,
            path: spec.path,
            kind: spec.kind,
            ...(spec.width !== undefined && { width: spec.width }),
            ...(spec.height !== undefined && { height: spec.height }),
            ...(spec.shape !== undefined && { shape: spec.shape }),
            class: item.class,
            order: item.order,
            ...(item.etag !== null && { etag: item.etag }),
        };
    }

    #send(message: ClientMessage): void {
        this.#socket?.send(JSON.stringify(message));
    }

    #connect(): void {
        let socket: ChannelSocket;
        try {
            socket = this.#env.open();
        } catch {
            this.#scheduleReconnect();
            return;
        }
        socket.binaryType = "arraybuffer";
        this.#socket = socket;
        this.#open = false;
        socket.onopen = () => {
            if (this.#socket !== socket) return;
            this.#open = true;
            this.#backoff = BACKOFF_FIRST_MS;
            this.#flush();
        };
        socket.onmessage = (event) => {
            if (this.#socket === socket) this.#receive(event.data);
        };
        socket.onerror = () => {};
        socket.onclose = (event) => {
            if (this.#socket === socket) this.#closed(event.code);
        };
    }

    #detach(): void {
        const socket = this.#socket;
        if (socket !== undefined) {
            socket.onopen = socket.onmessage = socket.onclose = socket.onerror = null;
        }
        this.#socket = undefined;
        this.#open = false;
    }

    /// つなぎ直すときのために、送った項目を送っていない状態へ戻す。daemon は閉じた接続の項目を覚えていない。
    #closed(code: number): void {
        this.#detach();
        this.#assembling.clear();
        this.#drops = [];
        for (const item of this.#items.values()) {
            if (item.phase === "sent") {
                item.phase = "pending";
                item.sent = undefined;
            }
            if (item.id !== undefined) this.#byId.delete(item.id);
            item.id = undefined;
        }
        if (code === CLOSE_SESSION_ENDED) {
            this.#env.onSessionEnded();
            return;
        }
        if (code === CLOSE_BROKE_CONTRACT) {
            this.#broken = true;
            for (const item of [...this.#items.values()]) {
                if (item.phase !== "done" && item.phase !== "looking") this.#fail(item, "InvalidRequest");
            }
            return;
        }
        if ([...this.#items.values()].some((item) => item.phase === "pending")) this.#scheduleReconnect();
    }

    /// 1 秒から始めて倍にし、30 秒までにする。複数のタブが同時につなぎ直さないよう、ばらつきを付ける。
    #scheduleReconnect(): void {
        if (this.#reconnect !== undefined) return;
        const wait = Math.min(this.#backoff, BACKOFF_MAX_MS) * (0.75 + 0.5 * this.#env.random());
        this.#backoff = Math.min(this.#backoff * 2, BACKOFF_MAX_MS);
        this.#reconnect = this.#env.setTimeout(() => {
            this.#reconnect = undefined;
            this.#connect();
        }, wait);
    }

    #receive(data: unknown): void {
        if (typeof data === "string") {
            const message = parseServerMessage(data);
            if (message === undefined) return;
            const item = this.#byId.get(message.id);
            if (item === undefined) return;
            if (message.type === "unchanged") {
                this.#settle(item, 0, true);
            } else {
                this.#failed(item, message.kind);
            }
        } else if (data instanceof ArrayBuffer) {
            const frame = decodeFrame(data);
            if (frame !== undefined) this.#chunk(frame.header, frame.chunk);
        }
    }

    #chunk(header: FrameHeader, chunk: Uint8Array): void {
        const item = this.#byId.get(header.id);
        if (item === undefined) return;
        const assembly = this.#assembling.get(header.id) ?? { header, parts: [], received: 0 };
        // 欠けたり重なったりした画像は組み立てない。つなぎ直したときに取り直す。
        if (header.offset !== assembly.received) {
            this.#assembling.delete(header.id);
            return;
        }
        assembly.parts.push(chunk);
        assembly.received += chunk.length;
        if (assembly.received < header.total) {
            this.#assembling.set(header.id, assembly);
            return;
        }
        this.#assembling.delete(header.id);
        const blob = new Blob(assembly.parts as BlobPart[], { type: header.contentType });
        this.#revoke(item);
        item.state = { status: "ready", url: this.#env.createUrl(blob), frameCount: header.frameCount };
        item.etag = header.etag;
        this.#settle(item, header.total, false);
        // 鍵のない画像は、最新かを確かめられないので残さない。
        if (header.etag !== null) {
            const { key } = item;
            const group = cacheGroupOf(item.spec.kind);
            void blob
                .arrayBuffer()
                .then((data) =>
                    this.#env.cache.put(key, group, {
                        data,
                        contentType: header.contentType,
                        etag: header.etag,
                        frameCount: header.frameCount,
                    }),
                )
                .catch(() => undefined);
        }
    }

    /// 項目の画像が出そろった。daemon の項目は、これで終わる。
    #settle(item: Item, bytes: number, cached: boolean): void {
        item.phase = "done";
        this.#release(item);
        this.#emit();
        this.#env.onSettled?.({
            spec: item.spec,
            durationMs: Math.max(0, this.#env.now() - item.startedAt),
            bytes,
            cached,
        });
    }

    #release(item: Item): void {
        if (item.id !== undefined) {
            this.#byId.delete(item.id);
            this.#assembling.delete(item.id);
        }
        item.id = undefined;
    }

    /// daemon が画像を作れないと答えた。手元に画像があれば、見つからないとき以外はそのまま出す。
    #failed(item: Item, kind: ErrorKind): void {
        if (item.state.status === "ready" && kind !== "NotFound") {
            item.phase = "done";
            this.#release(item);
            this.#emit();
            return;
        }
        if (kind === "NotFound") void this.#env.cache.delete(item.key).catch(() => undefined);
        this.#fail(item, kind);
    }

    #fail(item: Item, kind: ErrorKind): void {
        this.#revoke(item);
        item.state = { status: "failed", kind };
        item.phase = "done";
        this.#release(item);
        this.#trimFailed();
        this.#emit();
    }

    #trimFailed(): void {
        const failed = [...this.#items.values()].filter((item) => item.state.status === "failed");
        for (const item of failed.slice(0, Math.max(0, failed.length - MAX_FAILED))) {
            if (this.#best(item.key) === undefined) this.#items.delete(item.key);
        }
    }

    #emit(): void {
        for (const listener of this.#listeners) listener();
    }
}

/// ブラウザの WebSocket と IndexedDB で動く窓口を作る。`onSessionEnded` は、セッションが終わったと言われたときに呼ぶ。
export function createBrowserImageChannel(onSessionEnded: () => void): ImageChannelClient {
    const url = `${globalThis.location.protocol === "https:" ? "wss" : "ws"}://${globalThis.location.host}/api/v1/image-channel`;
    return new ImageChannelClient({
        open: () => new WebSocket(url),
        cache: createBrowserImageCache(),
        createUrl: (blob) => URL.createObjectURL(blob),
        revokeUrl: (url) => URL.revokeObjectURL(url),
        now: () => performance.now(),
        random: () => Math.random(),
        setTimeout: (callback, ms) => setTimeout(callback, ms),
        clearTimeout: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>),
        onSessionEnded,
        onSettled: ({ spec, durationMs, bytes, cached }) => {
            // 表示用画像の時間は、ビューアが、画像を出すまでを含めて測る。
            if (spec.kind !== "thumbnail") return;
            clientTimings.record({
                operation: "thumbnail_image",
                libraryId: spec.libraryId,
                path: spec.path,
                durationMs: Math.round(durationMs * 10) / 10,
                bytes,
                cached,
            });
        },
    });
}
