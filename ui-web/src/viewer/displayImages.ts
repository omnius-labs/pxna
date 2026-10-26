/// 表示用画像の要求の優先度。サーバは `interactive` の生成を、先読みとサムネイルより先に始める。
export type DisplayPriority = "interactive" | "prefetch";

/// 表示用画像を取得し、`<img>` に渡せる blob の URL を返す。
export async function loadDisplayImage(url: string, priority: DisplayPriority, signal: AbortSignal): Promise<string> {
    const response = await fetch(url, { headers: { "Pxna-Priority": priority }, signal });
    if (!response.ok) {
        throw new Error(`display image request failed with status ${response.status}`);
    }
    return URL.createObjectURL(await response.blob());
}

export function revokeDisplayImage(url: string): void {
    URL.revokeObjectURL(url);
}

interface Request {
    priority: DisplayPriority;
    controller: AbortController;
    promise: Promise<string>;
    /// 読み終えた画像の blob の URL。
    objectUrl?: string;
}

/// ビューアの表示用画像の要求（表示中の画像と、前後の先読み）。
///
/// 表示中の画像は `interactive`、先読みは `prefetch` で要求する。先読み中の画像を表示するときは、先読みを中止して
/// `interactive` で取り直すので、サーバで生成を待っている要求が前に出る。読み終えた先読みは、そのまま使う。
/// 残す画像から外れた要求は、読み込み中なら中止する。サーバは、生成の枠を待っている間に中止された要求の生成を始めない。
/// 読み終えた画像は、戻ったときに使えるよう、新しいものから `KEPT_LOADED` 枚まで残す。
export class DisplayImages {
    static readonly KEPT_LOADED = 6;

    readonly #load: typeof loadDisplayImage;
    readonly #revoke: (url: string) => void;
    readonly #requests = new Map<string, Request>();

    constructor(load: typeof loadDisplayImage, revoke: (url: string) => void) {
        this.#load = load;
        this.#revoke = revoke;
    }

    prefetch(url: string): void {
        if (!this.#requests.has(url)) {
            this.#start(url, "prefetch");
        }
    }

    /// 表示する画像の blob の URL を返す。
    show(url: string): Promise<string> {
        const request = this.#requests.get(url);
        if (request && (request.priority === "interactive" || request.objectUrl !== undefined)) {
            return request.promise;
        }
        request?.controller.abort();
        return this.#start(url, "interactive").promise;
    }

    /// `urls` に含まれない読み込み中の要求を中止し、古い読み終えた画像を解放する。
    retain(urls: Iterable<string>): void {
        const kept = new Set(urls);
        const loaded: string[] = [];
        for (const [url, request] of [...this.#requests]) {
            if (kept.has(url)) {
                // 残す画像を、新しいものとして末尾へ並べ直す。
                this.#requests.delete(url);
                this.#requests.set(url, request);
            } else if (request.objectUrl === undefined) {
                this.#drop(url, request);
            } else {
                loaded.push(url);
            }
        }
        for (const url of loaded.slice(0, Math.max(0, loaded.length - DisplayImages.KEPT_LOADED))) {
            const request = this.#requests.get(url);
            if (request) this.#drop(url, request);
        }
    }

    dispose(): void {
        for (const [url, request] of [...this.#requests]) {
            this.#drop(url, request);
        }
    }

    #start(url: string, priority: DisplayPriority): Request {
        const controller = new AbortController();
        const request: Request = { priority, controller, promise: Promise.resolve("") };
        request.promise = this.#load(url, priority, controller.signal).then((objectUrl) => {
            if (this.#requests.get(url) === request) {
                request.objectUrl = objectUrl;
            } else {
                // 読み終える前に外れた要求の画像は、使わずに解放する。
                this.#revoke(objectUrl);
            }
            return objectUrl;
        });
        // 先読みの失敗は表示しない。表示するときに取り直す。
        request.promise.catch(() => {
            if (this.#requests.get(url) === request && request.priority === "prefetch") {
                this.#requests.delete(url);
            }
        });
        this.#requests.set(url, request);
        return request;
    }

    #drop(url: string, request: Request): void {
        request.controller.abort();
        if (request.objectUrl !== undefined) {
            this.#revoke(request.objectUrl);
        }
        this.#requests.delete(url);
    }
}
