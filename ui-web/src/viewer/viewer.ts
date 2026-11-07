import { type Entry, entryKind } from "../library/entries";
import { type Listing, ListingSnapshotChanged, PAGE_SIZE } from "../library/listing";
import type { DisplaySet } from "../library/preferences";
import { joinPath, pathSegments } from "../location";

export interface IndexedEntry {
    index: number;
    entry: Entry;
}

/// 相対パスを持つ URL から開けるよう、Entry の位置をサーバーに聞く。
export async function findEntry(
    listing: Listing,
    base: string,
    view: string,
    signal: AbortSignal,
): Promise<IndexedEntry | undefined> {
    const path = joinPath([...pathSegments(base), ...pathSegments(view)]);
    signal.throwIfAborted();
    // Answer from the loaded range when it holds the Entry.
    if (listing.totalCount === undefined) await listing.loadPages(listing.locateRemote ? [0] : [], signal);
    signal.throwIfAborted();
    const loaded = listing.locate?.(path);
    if (loaded !== undefined) {
        const entry = listing.entryAt(loaded);
        if (entry !== undefined) return { index: loaded, entry };
    }
    // An Entry on an unread page is found by asking the server for its page.
    if (listing.locateRemote) {
        const index = await listing.locateRemote(path, signal);
        signal.throwIfAborted();
        const entry = index === undefined ? undefined : listing.entryAt(index);
        return index === undefined || entry === undefined ? undefined : { index, entry };
    }
    return undefined;
}

/// 前後への移動で止まる Entry か。ビューアで開ける画像と動画に止まり、それ以外は飛ばす。
export function isViewable(entry: Entry): boolean {
    const kind = entryKind(entry);
    return kind === "image" || kind === "video";
}

/// 前後の画像か動画へ移る。未読のページも読み終えてから移る。
export async function neighbor(
    listing: Listing,
    index: number,
    step: -1 | 1,
    signal: AbortSignal,
): Promise<IndexedEntry | undefined> {
    const snapshot = listing.snapshot;
    for (let next = index + step; next >= (listing.firstIndex ?? 0) && next < (listing.totalCount ?? 0); next += step) {
        await readIndex(listing, next, signal, snapshot);
        const entry = listing.entryAt(next);
        if (entry && isViewable(entry)) return { index: next, entry };
    }
    signal.throwIfAborted();
    return undefined;
}

/// Finds the image or video nearest a position, looking outward on both sides and reading pages as it goes.
/// At equal distances, the one after the position wins.
export async function viewableNear(
    listing: Listing,
    index: number,
    signal: AbortSignal,
): Promise<IndexedEntry | undefined> {
    const snapshot = listing.snapshot;
    const first = listing.firstIndex ?? 0;
    const end = listing.totalCount ?? 0;
    for (let distance = 0; index + distance < end || index - distance >= first; distance += 1) {
        for (const position of distance === 0 ? [index] : [index + distance, index - distance]) {
            if (position < first || position >= end) continue;
            await readIndex(listing, position, signal, snapshot);
            const entry = listing.entryAt(position);
            if (entry && isViewable(entry)) return { index: position, entry };
        }
    }
    signal.throwIfAborted();
    return undefined;
}

/// 先読みで一覧の未読のページを増やさない。未読なら移動の操作を待ち、端まで画像も動画もなければボタンを無効にする。
export function loadedNeighbor(listing: Listing, index: number, step: -1 | 1): IndexedEntry | "unread" | undefined {
    for (let next = index + step; next >= (listing.firstIndex ?? 0) && next < (listing.totalCount ?? 0); next += step) {
        const entry = listing.entryAt(next);
        if (!entry) return "unread";
        if (isViewable(entry)) return { index: next, entry };
    }
    return undefined;
}

/// 位置の Entry を読む。位置を読めば足りる一覧は、ページ全体を読まない。
async function readIndex(listing: Listing, index: number, signal: AbortSignal, snapshot?: string) {
    if (!listing.ensureIndex) return readPage(listing, Math.floor(index / PAGE_SIZE), signal, snapshot);
    signal.throwIfAborted();
    if (snapshot !== undefined && listing.snapshot !== snapshot) throw new ListingSnapshotChanged();
    await listing.ensureIndex(index, signal);
    signal.throwIfAborted();
    if (snapshot !== undefined && listing.snapshot !== snapshot) throw new ListingSnapshotChanged();
}

async function readPage(listing: Listing, page: number, signal: AbortSignal, snapshot?: string) {
    signal.throwIfAborted();
    if (snapshot !== undefined && listing.snapshot !== snapshot) throw new ListingSnapshotChanged();
    if (listing.pageStatus(page) !== "loaded") await listing.loadPages([page], signal);
    signal.throwIfAborted();
    if (snapshot !== undefined && listing.snapshot !== snapshot) throw new ListingSnapshotChanged();
}

export type OriginalImageMode = DisplaySet["originalImage"];

/// 元の画像を取得する時機。`immediate` は表示用画像が出た直後、`onZoom` は表示用画像の画素より細かく拡大したとき、`never` は取得しない。
export type OriginalTiming = "immediate" | "onZoom" | "never";

/// ブラウザの `img` で表示できる形式。これ以外は、元の画像へ差し替えず、表示用画像のまま拡大する。
const BROWSER_IMAGE_TYPES = new Set(["image/jpeg", "image/png", "image/gif", "image/webp", "image/avif", "image/bmp"]);

/// 「自動」で、表示用画像の直後に元の画像を取得する大きさの上限（バイト）。
export const IMMEDIATE_ORIGINAL_MAX_BYTES = 30_000_000;

/// 画像の形式と大きさと表示設定から、元の画像を取得する時機を決める。
export function originalTiming(entry: Entry, mode: OriginalImageMode): OriginalTiming {
    if (!BROWSER_IMAGE_TYPES.has(entry.metadata?.contentType ?? "")) {
        return "never";
    }
    return mode === "auto" && (entry.metadata?.size ?? Infinity) <= IMMEDIATE_ORIGINAL_MAX_BYTES
        ? "immediate"
        : "onZoom";
}

/// API の幅と高さの上限。
const MAX_REQUEST_SIDE = 4096;

/// 表示用画像に要求する幅と高さ。画面の大きさに画素密度を掛ける。サーバが縦横比を保ってサイズ段階へ収める。
export function displayRequestSize(
    viewWidth: number,
    viewHeight: number,
    pixelRatio: number,
): { width: number; height: number } {
    const ratio = Math.max(1, pixelRatio);
    const side = (css: number) => Math.min(MAX_REQUEST_SIDE, Math.max(1, Math.round(css * ratio)));
    return { width: side(viewWidth), height: side(viewHeight) };
}

export const MIN_ZOOM = 1;
export const MAX_ZOOM = 16;

export function clampZoom(zoom: number): number {
    return Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
}

/// ホイールの回転量から、拡大率の変化を指数で求める。上へ回すと拡大する。
export function zoomAfterWheel(zoom: number, deltaY: number): number {
    return clampZoom(zoom * Math.exp(-deltaY * 0.0015));
}

/// 縦横比を保って `container` に収めた画像の幅。
export function fittedWidth(
    container: { width: number; height: number },
    natural: { width: number; height: number },
): number {
    if (natural.width <= 0 || natural.height <= 0) {
        return 0;
    }
    return Math.min(container.width, (container.height * natural.width) / natural.height);
}

/// 表示用画像の画素より細かく拡大したときに限り、元の画像が要る。拡大していない間は要らない。
export function needsOriginal({
    fittedWidth: fitted,
    zoom,
    pixelRatio,
    naturalWidth,
}: {
    fittedWidth: number;
    zoom: number;
    pixelRatio: number;
    naturalWidth: number;
}): boolean {
    return zoom > MIN_ZOOM && fitted * zoom * Math.max(1, pixelRatio) > naturalWidth;
}

/// 拡大して動かした位置が、画像を画面の外へ出しすぎないよう、動かせる範囲に収める。
export function clampPan(
    pan: { x: number; y: number },
    zoom: number,
    container: { width: number; height: number },
): { x: number; y: number } {
    const limitX = (container.width * (zoom - 1)) / 2;
    const limitY = (container.height * (zoom - 1)) / 2;
    return { x: Math.min(limitX, Math.max(-limitX, pan.x)), y: Math.min(limitY, Math.max(-limitY, pan.y)) };
}

export interface ViewTransform {
    zoom: number;
    pan: { x: number; y: number };
}

export const DOUBLE_TAP_ZOOM = 2;

/// Scales the view by `factor` while the content under `point` stays under it, then moves it by `shift`.
/// `point` and `shift` are in CSS px, and `point` is measured from the center of the container.
/// Zooming back to the whole view resets the position, and any other result is kept within the pan limits.
export function zoomAround(
    view: ViewTransform,
    factor: number,
    point: { x: number; y: number },
    container: { width: number; height: number },
    shift: { x: number; y: number } = { x: 0, y: 0 },
): ViewTransform {
    const zoom = clampZoom(view.zoom * factor);
    if (zoom === MIN_ZOOM) {
        return { zoom, pan: { x: 0, y: 0 } };
    }
    const ratio = zoom / view.zoom;
    const pan = {
        x: point.x - (point.x - view.pan.x) * ratio + shift.x,
        y: point.y - (point.y - view.pan.y) * ratio + shift.y,
    };
    return { zoom, pan: clampPan(pan, zoom, container) };
}

/// A double tap enlarges the whole view around the tapped point, and returns an enlarged view to the whole view.
export function doubleTapView(
    view: ViewTransform,
    point: { x: number; y: number },
    container: { width: number; height: number },
): ViewTransform {
    return view.zoom > MIN_ZOOM
        ? { zoom: MIN_ZOOM, pan: { x: 0, y: 0 } }
        : zoomAround(view, DOUBLE_TAP_ZOOM, point, container);
}

/// ホイールの回転量で拡大率を変え、動かした位置を、新しい拡大率で動かせる範囲へ収め直す。
/// 縮小して 1 に戻ると位置もリセットする。
export function zoomView(
    view: ViewTransform,
    deltaY: number,
    container: { width: number; height: number },
): ViewTransform {
    const zoom = zoomAfterWheel(view.zoom, deltaY);
    return zoom === MIN_ZOOM ? { zoom, pan: { x: 0, y: 0 } } : { zoom, pan: clampPan(view.pan, zoom, container) };
}
