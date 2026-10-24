import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Download, Ellipsis, Maximize, Minimize, Pause, Play, ThumbsDown, ThumbsUp, Trash2 } from "lucide-react";
import { type PointerEvent, useCallback, useEffect, useRef, useState } from "react";
import { toast } from "sonner";
import { type Entry, entryKind } from "../library/entries";
import { canMutate } from "../library/entryActions";
import { type Listing, type ListingSource, useListing } from "../library/listing";
import type { SortPreference } from "../library/preferences";
import { putRating, RATING_LABELS, type Rating, ratingQuery } from "../library/ratingTags";
import { keepRange, reachablePages, useTimelineStore } from "../library/timeline";
import { type BrowseLocation, CROSS_LIBRARY_SCOPE, relativePath, viewKey } from "../location";
import { useThumbnailQueue } from "../thumbnails/useThumbnailQueue";
import { FILMSTRIP_ITEM_WIDTH } from "../viewer/filmstrip";
import { displayImageUrl, fileContentUrl, originalImageUrl } from "../viewer/urls";
import { useFullscreen } from "../viewer/useFullscreen";
import { type SlideshowStep, useSlideshow } from "../viewer/useSlideshow";
import {
    clampPan,
    clampZoom,
    displayRequestSize,
    findEntry,
    fittedWidth,
    loadedNeighbor,
    needsOriginal,
    neighbor,
    type OriginalImageMode,
    type OriginalTiming,
    originalTiming,
    SLIDESHOW_KINDS,
    type StepKinds,
    type ViewTransform,
    zoomView,
} from "../viewer/viewer";
import { DeleteDialog } from "./DeleteDialog";
import { Filmstrip } from "./Filmstrip";
import { Modal } from "./Modal";
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from "./ui/dropdown-menu";
import { Toaster } from "./ui/sonner";
import { VideoPlayer } from "./VideoPlayer";

interface ViewerProps {
    /// ビューアの範囲と、前後へ移る基準のフォルダと、開いている Entry。検索では範囲が `CROSS_LIBRARY_SCOPE` で、基準は `/` である。
    location: BrowseLocation;
    /// 前後へ移る一覧の出どころ。
    source: ListingSource;
    /// 一覧の並べ替え。前後の Entry への移動は、グリッドと同じ並び順に従う。
    sort: SortPreference;
    /// 元の画像へ差し替える時機の表示設定。今の画面の幅の区分の組の値を渡す。
    originalImage: OriginalImageMode;
    /// 狭い画面か。狭い画面にはフィルムストリップとダウンロードと削除のボタンを出さない。
    narrow: boolean;
    /// 開いている Library が読み取り専用か。読み取り専用なら削除できない。
    libraryReadonly: boolean;
    /// 開く Entry の相対パス。`undefined` なら閉じる。同じフォルダの別の Entry へ移るときも使う。
    onChange: (name: string | undefined) => void;
}

/// ビューアの一覧の読み込みを、フィルムストリップの見えている範囲に必要なページとして受け取る。
function useWantedPages() {
    const [wantedPages, setWantedPages] = useState<ReadonlySet<number>>(new Set());
    const changeWantedPages = useCallback(
        (pages: ReadonlySet<number>) =>
            setWantedPages((previous) => ([...previous].join(",") === [...pages].join(",") ? previous : pages)),
        [],
    );
    return { wantedPages, changeWantedPages };
}

/// フォルダの一覧から前後へ移る、画像と動画のビューア。
export function Viewer({ sort, source, ...props }: ViewerProps) {
    const { wantedPages, changeWantedPages } = useWantedPages();
    const listing = useListing(source, sort, wantedPages);
    return (
        <ViewerView
            {...props}
            listing={listing}
            scope={[sort.key, sort.order, JSON.stringify(source)]}
            onPages={changeWantedPages}
        />
    );
}

interface TimelineViewerProps extends Omit<ViewerProps, "location" | "sort" | "source"> {
    /// タイムラインに並べている Library の id。
    libraryIds: readonly string[];
    /// 開く Entry の鍵（先頭の `/` を除く）。Library の id を含む。
    view: string;
}

/// タイムラインの順で前後へ移るビューア。開いたときの Entry を中央に含む範囲から読み、前後へ読み足す。
/// Entry は Library を横断するので、所在はタイムラインの範囲の鍵から求める。
export function TimelineViewer({ libraryIds, view, ...props }: TimelineViewerProps) {
    const [opened] = useState(view);
    const store = useTimelineStore(libraryIds, { kind: "around", path: viewKey(opened) });
    const { wantedPages, changeWantedPages } = useWantedPages();
    const path = viewKey(view);
    // フィルムストリップを遠くへ送っても、開いている Entry から離れすぎたページは読まない。
    const current = store.locate(path);
    const reachable = reachablePages(current, wantedPages);
    const pagesKey = [...reachable].join(",");
    // biome-ignore lint/correctness/useExhaustiveDependencies: reachable は pagesKey から作り直せる値で、内容の変化だけを gate にする
    useEffect(() => {
        const controller = new AbortController();
        if (reachable.size > 0) void store.loadPages([...reachable], controller.signal).catch(() => {});
        return () => controller.abort();
    }, [pagesKey, store]);
    // 読み直しで範囲が差し替わったときに、今の Entry の近くから読み直せるよう、位置の目印を渡す。
    useEffect(() => {
        store.setFocus(path);
    }, [store, path]);
    // 開いている Entry とフィルムストリップの範囲から遠い Entry を捨てて、長く前後へ移っても保持する量が増え続けないようにする。
    const keep = keepRange(current, reachable);
    // biome-ignore lint/correctness/useExhaustiveDependencies: version は読み込みが終わるたびに保持範囲を確かめ直させる gate である
    useEffect(() => {
        if (keep) store.trim(keep.first, keep.last);
    }, [store, keep?.first, keep?.last, store.version]);
    // 範囲に今の Entry がなければ、その Entry を中央に含む範囲へ読み直す。
    const missing = store.isLoaded && store.locate(path) === undefined;
    useEffect(() => {
        if (missing) store.recenter(path);
    }, [missing, store, path]);
    return (
        <ViewerView
            {...props}
            location={{ libraryId: CROSS_LIBRARY_SCOPE, path: "/", view }}
            listing={store}
            scope={["timeline"]}
            onPages={changeWantedPages}
        />
    );
}

interface ViewerViewProps extends Omit<ViewerProps, "sort" | "source"> {
    listing: Listing;
    /// 一覧の条件。変わったら、開いている Entry の位置を探し直す。
    scope: readonly unknown[];
    /// フィルムストリップの見えている範囲に必要なページ。一覧を読む側へ渡す。
    onPages: (pages: ReadonlySet<number>) => void;
}

/// 画像と動画のビューア。サーバが縮めた表示用画像を出し、表示設定で選んだ時機に元の画像へ差し替える。
function ViewerView({
    location,
    listing,
    scope,
    onPages: changeWantedPages,
    originalImage,
    narrow,
    libraryReadonly,
    onChange,
}: ViewerViewProps) {
    const name = location.view;
    const identity = JSON.stringify([location.libraryId, location.path, ...scope, name, listing.snapshot]);
    const [found, setFound] = useState<{ identity: string; index?: number; done: boolean; error?: string }>({
        identity,
        done: false,
    });
    const [moving, setMoving] = useState(false);
    const [preparingDeletion, setPreparingDeletion] = useState(false);
    const movement = useRef<AbortController | undefined>(undefined);
    if (found.identity !== identity) {
        setFound({ identity, done: false });
        setMoving(false);
        setPreparingDeletion(false);
    }
    const index = found.identity === identity ? found.index : undefined;
    const current = index === undefined ? undefined : listing.entryAt(index);
    const [detailOwner] = useState(() => ({}));
    const currentPath = current?.path;
    useEffect(() => {
        listing.setDetailPriority?.(detailOwner, currentPath ? [currentPath] : []);
        return () => listing.releaseDetailPriority?.(detailOwner);
    }, [listing, detailOwner, currentPath]);
    const previous = index === undefined ? undefined : loadedNeighbor(listing, index, -1);
    const next = index === undefined ? undefined : loadedNeighbor(listing, index, 1);

    /// URL の相対パスや版が変わったら、古い探索の結果で別の画像へ移らないよう止める。
    useEffect(() => {
        const controller = new AbortController();
        if (name !== undefined) {
            void findEntry(listing, location.path, name, controller.signal)
                .then((entry) => {
                    if (!controller.signal.aborted) setFound({ identity, index: entry?.index, done: true });
                })
                .catch(() => {
                    if (!controller.signal.aborted)
                        setFound({ identity, done: true, error: "一覧を読めませんでした。" });
                });
        }
        return () => {
            controller.abort();
            movement.current?.abort();
        };
    }, [identity, listing, name, location.path]);

    // フィルムストリップのサムネイルのキュー。一覧のタイルサイズによらず、1 つの位置の幅で切り抜く。
    const { queue: filmstripQueue } = useThumbnailQueue(location.libraryId, 1, false, "crop", FILMSTRIP_ITEM_WIDTH);

    // 画面の大きさは、開いたときの値で決める。
    const [screen] = useState(() => ({
        width: window.innerWidth,
        height: window.innerHeight,
        pixelRatio: window.devicePixelRatio,
    }));
    /// 表示用画像の要求の大きさ。拡大せずに元の画像へ差し替える画像は、すぐ差し替わるので画素密度を掛けない。
    const sizeOf = useCallback(
        (target: Entry) =>
            displayRequestSize(
                screen.width,
                screen.height,
                originalTiming(target, originalImage) === "immediate" ? 1 : screen.pixelRatio,
            ),
        [screen, originalImage],
    );

    /// 読み込み済みの隣の画像を先に要求し、移ったときの待ちを減らす。読み込み中の Image は保持する。
    const prefetched = useRef(new Map<string, HTMLImageElement>());
    useEffect(() => {
        for (const neighborEntry of [previous, next]) {
            if (neighborEntry === undefined || neighborEntry === "unread") {
                continue;
            }
            const url = displayImageUrl(location.libraryId, neighborEntry.entry.path, sizeOf(neighborEntry.entry));
            if (!prefetched.current.has(url)) {
                const image = new Image();
                image.src = url;
                prefetched.current.set(url, image);
            }
        }
        while (prefetched.current.size > 6) {
            const oldest = prefetched.current.keys().next().value;
            if (oldest === undefined) {
                break;
            }
            prefetched.current.delete(oldest);
        }
    }, [previous, next, location.libraryId, sizeOf]);

    // スライドショーの自動の移動。停止したときに、読み込み中のものも取り消す。
    const autoMovement = useRef<AbortController | undefined>(undefined);
    /// 前後へ移る。別の移動か、削除の確認の準備の最中は、始めない。中断されても、その移動が最後なら、`moving` を解除する。
    const step = async (
        direction: -1 | 1,
        auto = false,
        kinds: StepKinds | undefined = auto ? SLIDESHOW_KINDS : undefined,
    ): Promise<SlideshowStep> => {
        if (index === undefined || moving || preparingDeletion || deletion) return "busy";
        const controller = new AbortController();
        movement.current?.abort();
        movement.current = controller;
        if (auto) autoMovement.current = controller;
        setMoving(true);
        try {
            const entry = await neighbor(listing, index, direction, controller.signal, kinds);
            if (controller.signal.aborted) return "busy";
            if (!entry) return "end";
            onChange(relativePath(location.path, entry.entry.path));
            return "moved";
        } catch {
            if (controller.signal.aborted) return "busy";
            setFound((value) => ({ ...value, error: "次の範囲を読めませんでした。" }));
            return "end";
        } finally {
            if (movement.current === controller) setMoving(false);
        }
    };
    const stepForward = () => step(1);
    const stepBackward = () => step(-1);

    // レートモード。入っている間、←・→・↑ は評価と次の Entry への移動に使う。スライドショーとは同時に使わない。
    const client = useQueryClient();
    const [rateMode, setRateMode] = useState(false);
    // 表示用画像が出た（か、読めないと分かった）画像。スライドショーは、これが変わるたびに 5 秒を数え直す。
    const [settledPath, setSettledPath] = useState<string>();
    const slideshow = useSlideshow({
        settled: current !== undefined && settledPath === current.path ? current.path : undefined,
        advance: () => step(1, true),
    });
    const { running: slideshowRunning, start: startSlideshowTimer, stop: stopSlideshowTimer } = slideshow;
    /// スライドショーを止める。次の範囲を読み込み中の自動の移動も取り消し、読み終えても移らないようにする。
    const stopSlideshow = useCallback(() => {
        autoMovement.current?.abort();
        stopSlideshowTimer();
    }, [stopSlideshowTimer]);
    /// スライドショーを始める。削除の確認の準備中と確認中は、始めない。
    const startSlideshow = () => {
        if (preparingDeletion || deletion) return;
        setRateMode(false);
        startSlideshowTimer();
    };
    // 開いている Entry が画像でも動画でもなくなったか、見つからなかったら止める。読み込み中（`current` がない間）は止めない。
    const slideshowUnavailable =
        (current !== undefined && entryKind(current) !== "image" && entryKind(current) !== "video") ||
        (found.done && current === undefined);
    useEffect(() => {
        if (slideshowRunning && slideshowUnavailable) stopSlideshow();
    }, [slideshowRunning, slideshowUnavailable, stopSlideshow]);

    /// スライドショー中の動画が最後まで再生されたか、再生できないと分かったら、次へ進む。次がなければ止まる。
    const videoDone = () => {
        if (!slideshowRunning) return;
        void step(1, true).then((result) => {
            if (result === "end") stopSlideshow();
        });
    };

    // 外枠の要素は、全画面にする要素と、メニューを描く先に使う。`dialog` の要素は全画面にできない。
    const [outer, setOuter] = useState<HTMLElement | null>(null);
    const fullscreen = useFullscreen(outer);
    const isImage = current !== undefined && entryKind(current) === "image";
    /// スライドショーの操作を出すか。画像のほか、スライドショーが動画に差し掛かっている間も出す。
    const showsSlideshow = isImage || (slideshowRunning && current !== undefined && entryKind(current) === "video");
    // 削除の確認。移動先は、確認を出す前に今の一覧で決める。削除の後は一覧が読み直されて位置がずれるので、位置でなく相対パスで持つ。
    // `destination` がなければ、削除の後にビューアを閉じる。
    const [deletion, setDeletion] = useState<{ path: string; destination: string | undefined }>();
    if (narrow && deletion) {
        setDeletion(undefined);
    }
    // 狭い画面の「その他」のメニュー。画面が広くなったり、画像でなくなったりしたら、閉じたものとして扱う。
    const [menuOpen, setMenuOpen] = useState(false);
    const menuVisible = menuOpen && narrow && showsSlideshow;
    // 外枠の中の確認のダイアログやメニューが開いている間は、ビューアのキー操作を行わない。それぞれの Esc と矢印キーに任せる。
    const nestedOpen = menuVisible || deletion !== undefined;
    /// 閉じる操作。全画面の間は全画面だけを終え、ビューアを開いたままにする。
    const requestClose = () => {
        if (nestedOpen) return;
        if (fullscreen.active) void fullscreen.exit();
        else onChange(undefined);
    };

    /// 削除の確認を出す。スライドショーを止め、次の画像、なければ前の画像を、削除の後の移動先にする。
    const requestDeletion = async () => {
        if (index === undefined || current === undefined || moving || preparingDeletion || deletion) return;
        stopSlideshow();
        const controller = new AbortController();
        movement.current = controller;
        setPreparingDeletion(true);
        try {
            const target =
                (await neighbor(listing, index, 1, controller.signal)) ??
                (await neighbor(listing, index, -1, controller.signal));
            if (controller.signal.aborted) return;
            // 準備の間に始められたスライドショーがあっても、確認を出す時点で止める。
            stopSlideshow();
            setDeletion({
                path: current.path,
                destination: target === undefined ? undefined : relativePath(location.path, target.entry.path),
            });
        } catch {
            if (!controller.signal.aborted) setFound((value) => ({ ...value, error: "次の範囲を読めませんでした。" }));
        } finally {
            if (movement.current === controller) setPreparingDeletion(false);
        }
    };

    const ratable = current !== undefined && (entryKind(current) === "image" || entryKind(current) === "video");
    const rating = useQuery({ ...ratingQuery(location.libraryId, current?.path ?? ""), enabled: rateMode && ratable });
    /// 評価を記録し、次の Entry へ移る。記録は移動を待たせず、失敗したらトーストで知らせる。次は、画像と動画のどちらにも止まる。
    const rate = (value: Rating | undefined) => {
        if (current === undefined || !ratable) return;
        if (value !== undefined) {
            void putRating(client, location.libraryId, current.path, value).catch(() =>
                toast.error("評価を保存できませんでした。", { toasterId: VIEWER_TOASTER }),
            );
        }
        void step(1, false, SLIDESHOW_KINDS);
    };
    const toggleRateMode = () => {
        if (!rateMode) stopSlideshow();
        setRateMode(!rateMode);
    };

    // キー操作は、フォーカスがどこにあっても効くよう、window で受ける。無効にしたボタンでフォーカスを失っても、効き続ける。
    // ブラウザの戻る操作のショートカットなど、修飾キーつきの操作は奪わない。
    useEffect(() => {
        const keyDown = (event: KeyboardEvent) => {
            // Radix の確認のダイアログとメニューは、Esc を処理すると `preventDefault` する。
            if (event.defaultPrevented || event.altKey || event.ctrlKey || event.metaKey || nestedOpen) {
                return;
            }
            if (event.key === "Escape") {
                event.preventDefault();
                requestClose();
            } else if (event.key === "r" && !narrow && ratable) {
                event.preventDefault();
                toggleRateMode();
            } else if (
                rateMode &&
                ratable &&
                (event.key === "ArrowRight" || event.key === "ArrowLeft" || event.key === "ArrowUp")
            ) {
                event.preventDefault();
                rate(event.key === "ArrowRight" ? "good" : event.key === "ArrowLeft" ? "bad" : undefined);
            } else if (event.key === "ArrowLeft" && current && entryKind(current) === "image") {
                event.preventDefault();
                void stepBackward();
            } else if (event.key === "ArrowRight" && current && entryKind(current) === "image") {
                event.preventDefault();
                void stepForward();
            }
        };
        window.addEventListener("keydown", keyDown);
        return () => window.removeEventListener("keydown", keyDown);
    });

    return (
        <Modal label={name} onClose={requestClose} outerRef={setOuter}>
            <div className="flex items-center justify-between px-4 py-2 text-sm">
                <span className="flex min-w-0 items-center gap-2">
                    <span className="truncate">{name}</span>
                    {rateMode && ratable && <RatingBadge rating={rating.data} />}
                </span>
                <HeaderActions
                    rate={!narrow && ratable ? { active: rateMode, toggle: toggleRateMode } : undefined}
                    image={showsSlideshow}
                    narrow={narrow}
                    slideshow={{ running: slideshow.running, start: startSlideshow, stop: stopSlideshow }}
                    fullscreen={fullscreen}
                    download={
                        current && isImage
                            ? { href: fileContentUrl(location.libraryId, current.path), name: current.name }
                            : undefined
                    }
                    deletable={
                        current !== undefined &&
                        canMutate({ readonly: libraryReadonly }, current) &&
                        !moving &&
                        !preparingDeletion
                    }
                    onDelete={() => void requestDeletion()}
                    menuOpen={menuVisible}
                    onMenuOpenChange={setMenuOpen}
                    container={outer}
                    onClose={() => onChange(undefined)}
                />
            </div>
            <div className="relative min-h-0 flex-1">
                {current === undefined ? (
                    <p className="p-6 text-neutral-300">
                        {found.error ?? (found.done && !deletion ? "見つかりませんでした。" : "読み込み中…")}
                    </p>
                ) : entryKind(current) === "video" ? (
                    <VideoPlayer
                        key={current.path}
                        libraryId={location.libraryId}
                        path={current.path}
                        onDone={videoDone}
                    />
                ) : entryKind(current) !== "image" ? (
                    <p className="p-6 text-neutral-300">この種類のファイルは、まだ表示できません。</p>
                ) : (
                    <ViewerImage
                        key={current.path}
                        libraryId={location.libraryId}
                        entry={current}
                        size={sizeOf(current)}
                        timing={originalTiming(current, originalImage)}
                        onSettled={() => setSettledPath(current.path)}
                    />
                )}
                {current && found.error && (
                    <p role="alert" className="absolute bottom-2 left-2 text-sm text-neutral-300">
                        {found.error}
                    </p>
                )}
                {moving && (
                    <p role="status" className="absolute bottom-2 right-2 text-sm text-neutral-300">
                        読み込み中…
                    </p>
                )}
                {current && entryKind(current) === "image" ? (
                    <>
                        <button
                            type="button"
                            aria-label="前の画像"
                            disabled={previous === undefined || moving}
                            className="absolute left-2 top-1/2 -translate-y-1/2 rounded-full bg-white/10 px-3 py-2 text-2xl hover:bg-white/20 disabled:opacity-20"
                            onClick={stepBackward}
                        >
                            ‹
                        </button>
                        <button
                            type="button"
                            aria-label="次の画像"
                            disabled={next === undefined || moving}
                            className="absolute right-2 top-1/2 -translate-y-1/2 rounded-full bg-white/10 px-3 py-2 text-2xl hover:bg-white/20 disabled:opacity-20"
                            onClick={stepForward}
                        >
                            ›
                        </button>
                    </>
                ) : null}
            </div>
            <DeleteDialog
                target={
                    deletion
                        ? { libraryId: location.libraryId, items: new Map([[deletion.path, { folder: false }]]) }
                        : undefined
                }
                container={outer}
                // 失敗のときはビューアが開いたままなので、ビューアの中に出す。成功してビューアを閉じるときは、閉じた後も見えるよう、ルートの `Toaster` に出す。
                toasters={{
                    failure: VIEWER_TOASTER,
                    success: deletion?.destination === undefined ? undefined : VIEWER_TOASTER,
                }}
                onClose={() => setDeletion(undefined)}
                onDeleted={() => onChange(deletion?.destination)}
            />
            <Toaster id={VIEWER_TOASTER} position="top-center" />
            {!narrow && index !== undefined && current && entryKind(current) === "image" && (
                <Filmstrip
                    listing={listing}
                    base={location.path}
                    index={index}
                    queue={filmstripQueue}
                    onSelect={onChange}
                    onPages={changeWantedPages}
                />
            )}
        </Modal>
    );
}

/// ビューアが開いている間の結果のトーストを描く `Toaster` の `id`。外枠の要素の中に置き、モーダルと全画面の上に見せる。
const VIEWER_TOASTER = "viewer";

/// レートモードで、開いている Entry の評価を出す。未評価のときは操作の案内を出す。
function RatingBadge({ rating }: { rating: Rating | undefined }) {
    if (rating === "good" || rating === "bad") {
        const Icon = rating === "good" ? ThumbsUp : ThumbsDown;
        return (
            <span
                data-rating={rating}
                className="flex shrink-0 items-center gap-1 rounded bg-white/15 px-1.5 py-0.5 text-xs"
            >
                <Icon className="size-3" />
                {RATING_LABELS[rating]}
            </span>
        );
    }
    return (
        <span data-rating="none" className="shrink-0 text-xs text-neutral-300">
            ← 悪い　↑ 次へ　→ 良い
        </span>
    );
}

interface HeaderActionsProps {
    /// レートモードの入口。広い画面の画像と動画にだけ置く。
    rate: { active: boolean; toggle: () => void } | undefined;
    /// 開いている Entry が画像か。スライドショーと全画面表示は、画像のビューアにだけ置く。
    image: boolean;
    narrow: boolean;
    slideshow: { running: boolean; start: () => void; stop: () => void };
    fullscreen: { supported: boolean; active: boolean; toggle: () => Promise<void> };
    /// 開いている画像の元のファイル。広い画面の画像にだけ出す。
    download: { href: string; name: string } | undefined;
    /// 開いている画像を削除できるか。
    deletable: boolean;
    onDelete: () => void;
    menuOpen: boolean;
    onMenuOpenChange: (open: boolean) => void;
    /// メニューを描く先。モーダルの外枠の要素。
    container: HTMLElement | null;
    onClose: () => void;
}

/// ビューアのヘッダーの操作。広い画面はボタンを並べ、狭い画面は「その他」のメニューにまとめる。
function HeaderActions({
    rate,
    image,
    narrow,
    slideshow,
    fullscreen,
    download,
    deletable,
    onDelete,
    menuOpen,
    onMenuOpenChange,
    container,
    onClose,
}: HeaderActionsProps) {
    const slideshowLabel = slideshow.running ? "スライドショーを止める" : "スライドショー";
    const fullscreenLabel = fullscreen.active ? "全画面表示を終える" : "全画面表示";
    const toggleSlideshow = slideshow.running ? slideshow.stop : slideshow.start;
    const buttonClass = "rounded p-1.5 hover:bg-white/10";
    return (
        <div className="flex items-center gap-1">
            {rate && (
                <button
                    type="button"
                    aria-label="レートモード"
                    aria-pressed={rate.active}
                    title={
                        rate.active ? "レートモードを終える（R）" : "レートモード（R）: → 良い、← 悪い、↑ 評価せず次へ"
                    }
                    className={`${buttonClass} ${rate.active ? "bg-white/20" : ""}`}
                    onClick={rate.toggle}
                >
                    <ThumbsUp className="size-4" />
                </button>
            )}
            {image && narrow && (
                <DropdownMenu open={menuOpen} onOpenChange={onMenuOpenChange}>
                    <DropdownMenuTrigger asChild>
                        <button type="button" aria-label="その他" title="その他" className={buttonClass}>
                            <Ellipsis className="size-4" />
                        </button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="end" container={container}>
                        <DropdownMenuItem onSelect={toggleSlideshow}>{slideshowLabel}</DropdownMenuItem>
                        {fullscreen.supported && (
                            <DropdownMenuItem onSelect={() => void fullscreen.toggle()}>
                                {fullscreenLabel}
                            </DropdownMenuItem>
                        )}
                    </DropdownMenuContent>
                </DropdownMenu>
            )}
            {image && !narrow && (
                <>
                    <button
                        type="button"
                        aria-label={slideshowLabel}
                        title={slideshowLabel}
                        className={buttonClass}
                        onClick={toggleSlideshow}
                    >
                        {slideshow.running ? <Pause className="size-4" /> : <Play className="size-4" />}
                    </button>
                    {fullscreen.supported && (
                        <button
                            type="button"
                            aria-label={fullscreenLabel}
                            title={fullscreenLabel}
                            className={buttonClass}
                            onClick={() => void fullscreen.toggle()}
                        >
                            {fullscreen.active ? <Minimize className="size-4" /> : <Maximize className="size-4" />}
                        </button>
                    )}
                    {download && (
                        <a
                            aria-label="ダウンロード"
                            title="ダウンロード"
                            href={download.href}
                            download={download.name}
                            className={buttonClass}
                        >
                            <Download className="size-4" />
                        </a>
                    )}
                    <button
                        type="button"
                        aria-label="削除"
                        title="削除"
                        disabled={!deletable}
                        className={`${buttonClass} disabled:opacity-40`}
                        onClick={onDelete}
                    >
                        <Trash2 className="size-4" />
                    </button>
                </>
            )}
            <button
                type="button"
                aria-label="閉じる"
                className="rounded px-2 py-1 text-lg hover:bg-white/10"
                onClick={onClose}
            >
                ×
            </button>
        </div>
    );
}

interface ViewerImageProps {
    libraryId: string;
    entry: Entry;
    size: { width: number; height: number };
    timing: OriginalTiming;
    /// 表示用画像が出たか、読めないと分かったときに呼ぶ。
    onSettled: () => void;
}

/// 1 枚の画像と、その拡大と移動。Entry が変わると作り直し、拡大と元の画像の状態を捨てる。
function ViewerImage({ libraryId, entry, size, timing, onSettled }: ViewerImageProps) {
    // 画像を出す領域の要素と大きさ。ウィンドウの大きさが変わっても追う。
    const [stage, setStage] = useState<HTMLDivElement | null>(null);
    const [container, setContainer] = useState({ width: 0, height: 0 });
    const [{ zoom, pan }, setView] = useState<ViewTransform>({ zoom: 1, pan: { x: 0, y: 0 } });
    const [natural, setNatural] = useState<{ width: number; height: number } | undefined>(undefined);
    const [originalReady, setOriginalReady] = useState(false);
    const [originalFailed, setOriginalFailed] = useState(false);
    const [displayFailed, setDisplayFailed] = useState(false);
    const drag = useRef<{ x: number; y: number } | undefined>(undefined);

    useEffect(() => {
        if (!stage) {
            return;
        }
        const observer = new ResizeObserver(([observed]) =>
            setContainer({ width: observed?.contentRect.width ?? 0, height: observed?.contentRect.height ?? 0 }),
        );
        observer.observe(stage);
        return () => observer.disconnect();
    }, [stage]);

    // ホイールで拡大する。ページのスクロールに奪われないよう、passive にしない。
    useEffect(() => {
        if (!stage) {
            return;
        }
        const wheel = (event: WheelEvent) => {
            event.preventDefault();
            setView((current) => zoomView(current, event.deltaY, container));
        };
        stage.addEventListener("wheel", wheel, { passive: false });
        return () => stage.removeEventListener("wheel", wheel);
    }, [stage, container]);

    const zoomedBeyondDisplay =
        natural !== undefined &&
        needsOriginal({
            fittedWidth: fittedWidth(container, natural),
            zoom,
            pixelRatio: window.devicePixelRatio,
            naturalWidth: natural.width,
        });
    // 一度要求した元の画像は、縮小しても外さない。読み込み中の要素を作り直して、読み込み前の画像を見せることがないようにする。
    const [originalRequested, setOriginalRequested] = useState(false);
    if (timing === "onZoom" && zoomedBeyondDisplay && !originalRequested) {
        setOriginalRequested(true);
    }
    // 直後に差し替える画像は、表示用画像が出てから要求する。
    const wantsOriginal =
        !originalFailed && (timing === "immediate" ? natural !== undefined : timing === "onZoom" && originalRequested);

    const pointerDown = (event: PointerEvent) => {
        if (zoom > 1) {
            drag.current = { x: event.clientX - pan.x, y: event.clientY - pan.y };
            event.currentTarget.setPointerCapture(event.pointerId);
        }
    };
    const pointerMove = (event: PointerEvent) => {
        const origin = drag.current;
        if (origin) {
            const x = event.clientX - origin.x;
            const y = event.clientY - origin.y;
            setView((current) => ({ zoom: current.zoom, pan: clampPan({ x, y }, current.zoom, container) }));
        }
    };
    const pointerUp = () => {
        drag.current = undefined;
    };

    if (displayFailed) {
        return <p className="p-6 text-neutral-300">画像を読めませんでした。</p>;
    }

    return (
        <div
            ref={setStage}
            className={`absolute inset-0 overflow-hidden select-none ${zoom > 1 ? "cursor-grab" : ""}`}
            onPointerDown={pointerDown}
            onPointerMove={pointerMove}
            onPointerUp={pointerUp}
            onPointerCancel={pointerUp}
        >
            <div
                className="absolute inset-0"
                style={{ transform: `translate(${pan.x}px, ${pan.y}px) scale(${clampZoom(zoom)})` }}
            >
                <img
                    src={displayImageUrl(libraryId, entry.path, size)}
                    alt={entry.name}
                    draggable={false}
                    className="absolute inset-0 h-full w-full object-contain"
                    onLoad={(event) => {
                        setNatural({
                            width: event.currentTarget.naturalWidth,
                            height: event.currentTarget.naturalHeight,
                        });
                        onSettled();
                    }}
                    onError={() => {
                        setDisplayFailed(true);
                        onSettled();
                    }}
                />
                {wantsOriginal && (
                    // ブラウザが表示できない形式なら、差し替えずに表示用画像のまま拡大する。読み込み中は見せない。
                    <img
                        src={originalImageUrl(libraryId, entry.path)}
                        alt=""
                        aria-hidden="true"
                        draggable={false}
                        data-original=""
                        className={`absolute inset-0 h-full w-full object-contain ${originalReady ? "" : "invisible"}`}
                        onLoad={() => setOriginalReady(true)}
                        onError={() => setOriginalFailed(true)}
                    />
                )}
            </div>
        </div>
    );
}
