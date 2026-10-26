import { expect, it } from "vitest";

import { DisplayImages, type DisplayPriority } from "./displayImages";

/// 要求ごとに、解決と中止を外から観察できる読み込み。
function loader() {
    const calls: Array<{
        url: string;
        priority: DisplayPriority;
        signal: AbortSignal;
        resolve: (url: string) => void;
    }> = [];
    const revoked: string[] = [];
    const load = (url: string, priority: DisplayPriority, signal: AbortSignal) =>
        new Promise<string>((resolve, reject) => {
            signal.addEventListener("abort", () => reject(new DOMException("aborted", "AbortError")));
            calls.push({ url, priority, signal, resolve });
        });
    return { calls, revoked, images: new DisplayImages(load, (url) => revoked.push(url)) };
}

const settled = () => new Promise((resolve) => setTimeout(resolve, 0));

it("showing an image that is still prefetching aborts the prefetch and requests it as interactive", async () => {
    const { calls, images } = loader();
    images.prefetch("/a");
    const shown = images.show("/a");

    expect(calls.map(({ url, priority }) => [url, priority])).toEqual([
        ["/a", "prefetch"],
        ["/a", "interactive"],
    ]);
    expect(calls[0]?.signal.aborted).toBe(true);
    calls[1]?.resolve("blob:a");
    await expect(shown).resolves.toBe("blob:a");
});

it("showing a loaded prefetch reuses it without another request", async () => {
    const { calls, images } = loader();
    images.prefetch("/a");
    calls[0]?.resolve("blob:a");
    await settled();

    await expect(images.show("/a")).resolves.toBe("blob:a");
    expect(calls).toHaveLength(1);
});

it("retaining other images aborts the requests in progress and keeps recent loaded ones", async () => {
    const { calls, revoked, images } = loader();
    for (let index = 0; index < 8; index++) {
        images.prefetch(`/${index}`);
        calls[index]?.resolve(`blob:${index}`);
    }
    await settled();
    images.prefetch("/loading");
    images.retain(["/7"]);

    expect(calls.at(-1)?.signal.aborted).toBe(true);
    expect(revoked).toEqual(["blob:0"]);
    images.dispose();
    expect(revoked).toHaveLength(8);
});

it("an image that finishes after it was dropped is revoked at once", async () => {
    const revoked: string[] = [];
    let resolve: (url: string) => void = () => {};
    // 中止に気づかずに読み終える読み込み。
    const images = new DisplayImages(
        () =>
            new Promise<string>((done) => {
                resolve = done;
            }),
        (url) => revoked.push(url),
    );
    images.show("/a");
    images.dispose();
    resolve("blob:late");
    await settled();
    expect(revoked).toEqual(["blob:late"]);
});
