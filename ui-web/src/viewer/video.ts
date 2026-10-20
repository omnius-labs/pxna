import { api, unwrap } from "../api/client";
import type { components } from "../api/schema";

export type VideoSession = components["schemas"]["VideoSession"];
export type PlaybackMode = "direct" | "native-hls" | "hls-js" | "unsupported";

export function playbackMode(
    contentType: string,
    canPlayType: (type: string) => string,
    hlsSupported: boolean,
): PlaybackMode {
    if (canPlayType(contentType)) return "direct";
    if (canPlayType("application/vnd.apple.mpegurl")) return "native-hls";
    return hlsSupported ? "hls-js" : "unsupported";
}

interface PendingSession {
    references: number;
    promise: Promise<VideoSession>;
}

const sessions = new Map<string, PendingSession>();
let creationQueue: Promise<unknown> = Promise.resolve();

/** StrictMode の再接続では同じ要求を使い、作成中に閉じられたセッションも応答後に終了する。 */
export function acquireVideoSession(
    libraryId: string,
    path: string,
): { session: Promise<VideoSession>; release: () => void } {
    const key = JSON.stringify([libraryId, path]);
    let pending = sessions.get(key);
    if (!pending) {
        const create = async () => {
            if (entry.references === 0) throw new DOMException("Playback ended", "AbortError");
            const result = await api.POST("/libraries/{libraryId}/video/sessions", {
                params: { path: { libraryId } },
                body: { path },
            });
            return unwrap(result).session;
        };
        // 作成順を保つ。閉じた動画の遅い応答が、新しく開いた動画のセッションを追い出すことを防ぐ。
        const promise = creationQueue.then(create, create);
        const entry: PendingSession = { references: 0, promise };
        creationQueue = promise.catch(() => undefined);
        sessions.set(key, entry);
        pending = entry;
    }
    const entry = pending;
    entry.references += 1;
    let released = false;
    return {
        session: entry.promise,
        release: () => {
            if (released) return;
            released = true;
            entry.references -= 1;
            queueMicrotask(() => {
                if (entry.references !== 0 || sessions.get(key) !== entry) return;
                sessions.delete(key);
                void entry.promise
                    .then((session) =>
                        api.DELETE("/libraries/{libraryId}/video/sessions/{sessionId}", {
                            params: { path: { libraryId, sessionId: session.id } },
                            keepalive: true,
                        }),
                    )
                    .catch(() => undefined);
            });
        },
    };
}
