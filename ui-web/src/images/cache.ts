import type { ImageKind } from "./spec";

/// 予算は 2 つの群で別々に持つ。大きい表示用画像が、一覧のサムネイルを押し出さないようにするためである。
export type CacheGroup = "thumbnail" | "display";

/// サムネイルとスプライトシートの予算と、表示用画像の予算（Client Design §4）。
export const CACHE_BUDGETS: Record<CacheGroup, number> = {
    thumbnail: 256 * 1024 * 1024,
    display: 512 * 1024 * 1024,
};

export function cacheGroupOf(kind: ImageKind): CacheGroup {
    return kind === "display" ? "display" : "thumbnail";
}

/// 手元に残す画像。`etag` は daemon が付けた鍵で、欲しいと言うときに添えて、今も最新かを確かめる。
export interface CachedImage {
    data: ArrayBuffer;
    contentType: string;
    etag: string | null;
    frameCount: number | null;
}

/// 受け取った画像の永続的な置き場。失敗しても画像は出せるので、失敗は呼び出し側に見せない。
export interface ImageCache {
    get(key: string): Promise<CachedImage | undefined>;
    put(key: string, group: CacheGroup, image: CachedImage): Promise<void>;
    delete(key: string): Promise<void>;
    clear(): Promise<void>;
}

interface Entry {
    group: CacheGroup;
    image: CachedImage;
}

/// 置き場が使えない環境と、テストのための、メモリだけの置き場。群ごとの予算を超えたら、使われていない順に捨てる。
export class MemoryImageCache implements ImageCache {
    readonly #budgets: Record<CacheGroup, number>;
    // Map は挿入の順を保つので、先頭が最も長く使われていない。
    readonly #entries = new Map<string, Entry>();

    constructor(budgets: Record<CacheGroup, number> = CACHE_BUDGETS) {
        this.#budgets = budgets;
    }

    async get(key: string): Promise<CachedImage | undefined> {
        const entry = this.#entries.get(key);
        if (entry === undefined) return undefined;
        this.#entries.delete(key);
        this.#entries.set(key, entry);
        return entry.image;
    }

    async put(key: string, group: CacheGroup, image: CachedImage): Promise<void> {
        this.#entries.delete(key);
        this.#entries.set(key, { group, image });
        let total = this.#total(group);
        for (const [other, entry] of this.#entries) {
            if (total <= this.#budgets[group]) break;
            if (other === key || entry.group !== group) continue;
            this.#entries.delete(other);
            total -= entry.image.data.byteLength;
        }
    }

    async delete(key: string): Promise<void> {
        this.#entries.delete(key);
    }

    async clear(): Promise<void> {
        this.#entries.clear();
    }

    /// 入っている件数。テストが確かめるために使う。
    get size(): number {
        return this.#entries.size;
    }

    #total(group: CacheGroup): number {
        let total = 0;
        for (const entry of this.#entries.values()) {
            if (entry.group === group) total += entry.image.data.byteLength;
        }
        return total;
    }
}

const DB_NAME = "pxna-images";
const DB_VERSION = 1;
const IMAGES = "images";
const USAGE = "usage";
const BY_GROUP_USED = "byGroupUsed";

interface StoredImage extends CachedImage {
    key: string;
}

/// 画像そのものとは別に持つ、使われた時刻と大きさ。使った印を付けるときに、画像を書き直さずに済む。
interface StoredUsage {
    key: string;
    group: CacheGroup;
    size: number;
    lastUsed: number;
}

function requestResult<T>(request: IDBRequest<T>): Promise<T> {
    return new Promise((resolve, reject) => {
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
    });
}

function transactionDone(transaction: IDBTransaction): Promise<void> {
    return new Promise((resolve, reject) => {
        transaction.oncomplete = () => resolve();
        transaction.onerror = () => reject(transaction.error);
        transaction.onabort = () => reject(transaction.error);
    });
}

/// IndexedDB に画像を残す。再読み込みや別のタブでも、見た画像をもう一度送らせずに済む。
/// 群ごとの予算を超えたら、最後に使った時刻が古いものから消す。操作は順に実行し、合計の数え違いを防ぐ。
export class IndexedDbImageCache implements ImageCache {
    readonly #factory: IDBFactory;
    readonly #budgets: Record<CacheGroup, number>;
    readonly #name: string;
    #database: Promise<IDBDatabase> | undefined;
    #totals: Record<CacheGroup, number> | undefined;
    #chain: Promise<unknown> = Promise.resolve();
    #clock = 0;

    constructor(factory: IDBFactory, budgets: Record<CacheGroup, number> = CACHE_BUDGETS, name = DB_NAME) {
        this.#factory = factory;
        this.#budgets = budgets;
        this.#name = name;
    }

    get(key: string): Promise<CachedImage | undefined> {
        return this.#serial(async (database) => {
            const transaction = database.transaction([IMAGES, USAGE], "readwrite");
            const stored = await requestResult<StoredImage | undefined>(transaction.objectStore(IMAGES).get(key));
            if (stored !== undefined) {
                const usage = await requestResult<StoredUsage | undefined>(transaction.objectStore(USAGE).get(key));
                if (usage !== undefined) {
                    transaction.objectStore(USAGE).put({ ...usage, lastUsed: this.#tick() });
                }
            }
            await transactionDone(transaction);
            return stored === undefined
                ? undefined
                : {
                      data: stored.data,
                      contentType: stored.contentType,
                      etag: stored.etag,
                      frameCount: stored.frameCount,
                  };
        });
    }

    put(key: string, group: CacheGroup, image: CachedImage): Promise<void> {
        return this.#serial(async (database) => {
            const totals = await this.#loadTotals(database);
            const transaction = database.transaction([IMAGES, USAGE], "readwrite");
            const previous = await requestResult<StoredUsage | undefined>(transaction.objectStore(USAGE).get(key));
            transaction.objectStore(IMAGES).put({ key, ...image } satisfies StoredImage);
            transaction.objectStore(USAGE).put({
                key,
                group,
                size: image.data.byteLength,
                lastUsed: this.#tick(),
            } satisfies StoredUsage);
            await transactionDone(transaction);
            if (previous !== undefined) totals[previous.group] -= previous.size;
            totals[group] += image.data.byteLength;
            if (totals[group] > this.#budgets[group]) await this.#evict(database, group, key, totals);
        });
    }

    delete(key: string): Promise<void> {
        return this.#serial(async (database) => {
            const totals = await this.#loadTotals(database);
            const transaction = database.transaction([IMAGES, USAGE], "readwrite");
            const usage = await requestResult<StoredUsage | undefined>(transaction.objectStore(USAGE).get(key));
            transaction.objectStore(IMAGES).delete(key);
            transaction.objectStore(USAGE).delete(key);
            await transactionDone(transaction);
            if (usage !== undefined) totals[usage.group] -= usage.size;
        });
    }

    clear(): Promise<void> {
        return this.#serial(async (database) => {
            const transaction = database.transaction([IMAGES, USAGE], "readwrite");
            transaction.objectStore(IMAGES).clear();
            transaction.objectStore(USAGE).clear();
            await transactionDone(transaction);
            this.#totals = { thumbnail: 0, display: 0 };
        });
    }

    #serial<T>(operation: (database: IDBDatabase) => Promise<T>): Promise<T> {
        const run = this.#chain.then(async () => operation(await this.#open()));
        this.#chain = run.catch(() => undefined);
        return run;
    }

    #open(): Promise<IDBDatabase> {
        this.#database ??= new Promise((resolve, reject) => {
            const request = this.#factory.open(this.#name, DB_VERSION);
            request.onupgradeneeded = () => {
                const database = request.result;
                database.createObjectStore(IMAGES, { keyPath: "key" });
                database.createObjectStore(USAGE, { keyPath: "key" }).createIndex(BY_GROUP_USED, ["group", "lastUsed"]);
            };
            request.onsuccess = () => resolve(request.result);
            request.onerror = () => reject(request.error);
        });
        return this.#database;
    }

    /// 使った時刻。同じ時刻に 2 つの印が付いても、順が決まるよう、増やし続ける。
    #tick(): number {
        this.#clock = Math.max(Date.now(), this.#clock + 1);
        return this.#clock;
    }

    async #loadTotals(database: IDBDatabase): Promise<Record<CacheGroup, number>> {
        if (this.#totals !== undefined) return this.#totals;
        const totals: Record<CacheGroup, number> = { thumbnail: 0, display: 0 };
        const usages = await requestResult<StoredUsage[]>(
            database.transaction(USAGE, "readonly").objectStore(USAGE).getAll(),
        );
        for (const usage of usages) totals[usage.group] += usage.size;
        this.#totals = totals;
        return totals;
    }

    /// 群の合計が予算に収まるまで、最後に使った時刻が古い順に消す。今入れた画像は消さない。
    async #evict(database: IDBDatabase, group: CacheGroup, keep: string, totals: Record<CacheGroup, number>) {
        const transaction = database.transaction([IMAGES, USAGE], "readwrite");
        const usage = transaction.objectStore(USAGE);
        const range = IDBKeyRange.bound([group, 0], [group, Number.MAX_SAFE_INTEGER]);
        await new Promise<void>((resolve, reject) => {
            const cursor = usage.index(BY_GROUP_USED).openCursor(range);
            cursor.onerror = () => reject(cursor.error);
            cursor.onsuccess = () => {
                const position = cursor.result;
                if (position === null || totals[group] <= this.#budgets[group]) return resolve();
                const stored = position.value as StoredUsage;
                if (stored.key !== keep) {
                    position.delete();
                    transaction.objectStore(IMAGES).delete(stored.key);
                    totals[group] -= stored.size;
                }
                position.continue();
            };
        });
        await transactionDone(transaction);
    }
}

/// ブラウザの IndexedDB が使えればそれを、使えなければメモリだけの置き場を返す。置き場の失敗は画像を妨げない。
export function createBrowserImageCache(): ImageCache {
    if (typeof indexedDB === "undefined") return new MemoryImageCache();
    return new GuardedImageCache(new IndexedDbImageCache(indexedDB));
}

/// 置き場の失敗を握りつぶす。IndexedDB が壊れていても、画像は daemon から受け取って出せる。
class GuardedImageCache implements ImageCache {
    readonly #inner: ImageCache;

    constructor(inner: ImageCache) {
        this.#inner = inner;
    }

    get(key: string) {
        return this.#inner.get(key).catch(() => undefined);
    }

    put(key: string, group: CacheGroup, image: CachedImage) {
        return this.#inner.put(key, group, image).catch(() => undefined);
    }

    delete(key: string) {
        return this.#inner.delete(key).catch(() => undefined);
    }

    clear() {
        return this.#inner.clear().catch(() => undefined);
    }
}
