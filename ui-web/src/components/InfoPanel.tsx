import { useQuery, useQueryClient } from "@tanstack/react-query";
import { PanelRight } from "lucide-react";
import { type ReactNode, useEffect, useSyncExternalStore } from "react";

import { ENTRY_KIND_LABELS, type Entry, entryKind, isThumbnailable } from "../library/entries";
import {
    formatBytes,
    formatCamera,
    formatDimensions,
    formatDuration,
    formatExposureTime,
    formatFNumber,
    formatFocalLength,
    formatModifiedAt,
    formatTakenAt,
} from "../library/format";
import { INFO_PANEL_THUMBNAIL_SIDE } from "../library/infoPanel";
import { retryEntryDetails } from "../library/listing";
import { librariesQuery, type MediaInfo, mediaInfoQuery } from "../library/queries";
import { type Selection, selectionBytes } from "../library/selection";
import { useSettled } from "../library/useSettled";
import { CROSS_LIBRARY_SCOPE, entryAddress } from "../location";
import type { ThumbnailState } from "../thumbnails/queue";
import { useThumbnailQueue } from "../thumbnails/useThumbnailQueue";
import { EntryIcon } from "./EntryIcon";
import { EntryTags } from "./EntryTags";
import { Button } from "./ui/button";
import { Separator } from "./ui/separator";

/// ツールバーの、情報パネルを開閉するボタン。
export function InfoPanelToggle({ open, onToggle }: { open: boolean; onToggle: () => void }) {
    return (
        <Button
            variant="ghost"
            size="icon"
            aria-label="情報パネル"
            aria-pressed={open}
            title={open ? "情報パネルを畳む" : "情報パネルを開く"}
            onClick={onToggle}
        >
            <PanelRight />
        </Button>
    );
}

interface InfoPanelProps {
    libraryId: string;
    selection: Selection;
}

const IDLE: ThumbnailState = { status: "idle" };

/// 一覧の右に置く、選んだ Entry の詳細の表示。
export function InfoPanel({ libraryId, selection }: InfoPanelProps) {
    const client = useQueryClient();
    const items = [...selection.items.values()];
    const [only] = items;
    // 大きめのサムネイルは、1 件を選んでいて、選択が 200 ms 変わらなかったときにだけ要求する。
    const settled = useSettled(items.length === 1 ? only?.entry : undefined);
    const { queue } = useThumbnailQueue(libraryId, 1, false, "fit", INFO_PANEL_THUMBNAIL_SIDE);
    useEffect(() => {
        queue.setWanted(settled !== undefined && isThumbnailable(settled) ? new Map([[settled.path, 0]]) : new Map());
    }, [queue, settled]);
    const thumbnail = useSyncExternalStore(queue.subscribe, () =>
        settled === undefined ? IDLE : queue.getState(settled.path),
    );
    // メディア情報も同じ規則で、画像と動画だけを要求する。
    const mediaTarget = settled !== undefined && hasMediaInfo(settled) ? settled : undefined;
    const media = useQuery({
        ...mediaInfoQuery(libraryId, mediaTarget?.path ?? ""),
        enabled: mediaTarget !== undefined,
    });
    if (only === undefined) {
        return <p className="p-4 text-sm text-muted-foreground">Entry を選ぶと、詳細が出ます。</p>;
    }
    if (items.length > 1) {
        return <SelectionSummary count={items.length} bytes={selectionBytes(selection)} />;
    }
    return (
        <EntryDetail
            entry={only.entry}
            onRetry={() => retryEntryDetails(client, libraryId, [only.entry.path])}
            thumbnailUrl={thumbnail.status === "ready" ? thumbnail.url : undefined}
            libraryId={libraryId}
            tagsTarget={mediaTarget}
            media={mediaTarget === undefined ? undefined : media}
        />
    );
}

/// 複数を選んでいるときの、件数と合計のサイズ。合計はステータスバーと同じ値である。
function SelectionSummary({ count, bytes }: { count: number; bytes: number | undefined }) {
    return (
        <div className="p-4">
            <h2 className="font-heading text-sm font-semibold">{count} 件を選択中</h2>
            <p className="text-xs text-muted-foreground">合計 {formatBytes(bytes)}</p>
        </div>
    );
}

/// 1 件を選んでいるときの、名前と、種類、サイズ、更新日時、Library 相対パス。
function hasMediaInfo(entry: Entry): boolean {
    const kind = entryKind(entry);
    return kind === "image" || kind === "video";
}

interface MediaState {
    isPending: boolean;
    isError: boolean;
    data: MediaInfo | undefined;
}

function EntryDetail({
    entry,
    onRetry,
    thumbnailUrl,
    libraryId,
    tagsTarget,
    media,
}: {
    entry: Entry;
    libraryId: string;
    /// 評価とタグを出す Entry。選択が落ち着いた画像と動画だけで、メディア情報と同じ規則で決める。
    tagsTarget: Entry | undefined;
    onRetry(): void;
    thumbnailUrl: string | undefined;
    media: MediaState | undefined;
}) {
    const kind = entryKind(entry);
    const hasSize = kind !== "folder" && kind !== "archive";
    // タイムラインの Entry は Library を横断するので、Library の名前と、Library 相対パスを分けて出す。
    const address = entryAddress(libraryId, entry.path);
    const libraries = useQuery({ ...librariesQuery, enabled: libraryId === CROSS_LIBRARY_SCOPE });
    const libraryName = libraries.data?.find((library) => library.id === address.libraryId)?.name ?? address.libraryId;
    return (
        <>
            <div className="p-4">
                <div className="flex h-56 items-center justify-center overflow-hidden rounded-lg bg-muted text-muted-foreground">
                    {thumbnailUrl === undefined ? (
                        <EntryIcon kind={kind} className="size-16" />
                    ) : (
                        <img src={thumbnailUrl} alt="" draggable={false} className="max-h-56 w-full object-contain" />
                    )}
                </div>
                <h2 className="mt-3 truncate font-heading text-sm font-semibold" title={entry.name}>
                    {entry.name}
                </h2>
            </div>
            <Separator />
            <dl className="px-4 py-3">
                <InfoRow label="種類" value={ENTRY_KIND_LABELS[kind]} />
                {hasSize && <InfoRow label="サイズ" value={formatBytes(entry.metadata?.size)} />}
                <InfoRow label="更新日時" value={formatModifiedAt(entry.metadata?.modifiedAt)} />
                {libraryId === CROSS_LIBRARY_SCOPE && <InfoRow label="Library" value={libraryName} />}
                <InfoRow label="場所" value={address.path} />
            </dl>
            {entry.detailError && (
                <p role="alert" className="px-4 pb-3 text-xs text-muted-foreground">
                    詳細を取得できませんでした。
                    <Button variant="link" onClick={onRetry}>
                        再試行
                    </Button>
                </p>
            )}
            {media?.isPending && <p className="px-4 pb-3 text-xs text-muted-foreground">読み込み中…</p>}
            {media?.isError && (
                <p role="alert" className="px-4 pb-3 text-xs text-muted-foreground">
                    メディア情報を読めませんでした。
                </p>
            )}
            {media?.data && <MediaDetail info={media.data} />}
            {tagsTarget && (
                <>
                    <Separator />
                    <EntryTags key={tagsTarget.path} libraryId={libraryId} path={tagsTarget.path} />
                </>
            )}
        </>
    );
}

function InfoRow({ label, value }: { label: string; value: ReactNode }) {
    return (
        <div className="grid grid-cols-[5.5rem_1fr] gap-2 py-1 text-xs">
            <dt className="text-muted-foreground">{label}</dt>
            <dd className="min-w-0 break-words">{value}</dd>
        </div>
    );
}

/// 画像の寸法と撮影情報、動画の長さと解像度とコーデック。撮影情報は、応答が持つ項目だけを出す。
function MediaDetail({ info }: { info: MediaInfo }) {
    if (info.kind === "video" && info.video) {
        const { video } = info;
        return (
            <>
                <Separator />
                <dl className="px-4 py-3">
                    <InfoRow label="長さ" value={formatDuration(video.durationSecs)} />
                    <InfoRow label="解像度" value={formatDimensions(video.width, video.height)} />
                    <InfoRow
                        label="コーデック"
                        value={video.audioCodec ? `${video.videoCodec} / ${video.audioCodec}` : video.videoCodec}
                    />
                </dl>
            </>
        );
    }
    if (info.kind === "image" && info.image) {
        const { image } = info;
        const { shooting } = image;
        const rows: [string, string | undefined][] = [
            ["撮影日時", shooting.takenAt === undefined ? undefined : formatTakenAt(shooting.takenAt)],
            ["カメラ", formatCamera(shooting.cameraMake, shooting.cameraModel)],
            ["レンズ", shooting.lensModel],
            ["焦点距離", shooting.focalLengthMm === undefined ? undefined : formatFocalLength(shooting.focalLengthMm)],
            ["絞り値", shooting.fNumber === undefined ? undefined : formatFNumber(shooting.fNumber)],
            [
                "露出時間",
                shooting.exposureTimeSecs === undefined ? undefined : formatExposureTime(shooting.exposureTimeSecs),
            ],
            ["ISO 感度", shooting.iso === undefined ? undefined : String(shooting.iso)],
        ];
        const present = rows.filter((row): row is [string, string] => row[1] !== undefined);
        return (
            <>
                <Separator />
                <dl className="px-4 py-3">
                    <InfoRow label="寸法" value={formatDimensions(image.width, image.height, image.rotation)} />
                </dl>
                {present.length > 0 && (
                    <>
                        <Separator />
                        <div className="px-4 py-3">
                            <h3 className="mb-1 text-xs font-medium">撮影情報</h3>
                            <dl>
                                {present.map(([label, value]) => (
                                    <InfoRow key={label} label={label} value={value} />
                                ))}
                            </dl>
                        </div>
                    </>
                )}
            </>
        );
    }
    return null;
}
