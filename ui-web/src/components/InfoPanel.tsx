import { useQuery, useQueryClient } from "@tanstack/react-query";
import { t } from "i18next";
import { PanelRight } from "lucide-react";
import { type ReactNode, useEffect, useRef, useState, useSyncExternalStore } from "react";
import { IDLE } from "../images/spec";
import { useImageWants } from "../images/wants";
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
import { infoThumbnailSide } from "../library/infoPanel";
import { retryEntryDetails } from "../library/listing";
import { INFO_THUMBNAIL_HEIGHT, INFO_THUMBNAIL_HEIGHT_KEY, usePanelWidth } from "../library/panelWidth";
import { librariesQuery, type MediaInfo, mediaInfoQuery } from "../library/queries";
import { type Selection, selectionBytes } from "../library/selection";
import { useSettled } from "../library/useSettled";
import { CROSS_LIBRARY_SCOPE, entryAddress } from "../location";
import { EntryIcon } from "./EntryIcon";
import { EntryTags } from "./EntryTags";
import { ResizeHandle } from "./ResizeHandle";
import { Button } from "./ui/button";
import { Separator } from "./ui/separator";

/// ツールバーの、情報パネルを開閉するボタン。
export function InfoPanelToggle({ open, onToggle }: { open: boolean; onToggle: () => void }) {
    return (
        <Button
            variant="ghost"
            size="icon"
            aria-label={t("info.label")}
            aria-pressed={open}
            title={open ? t("info.collapse") : t("info.open")}
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

/// 一覧の右に置く、選んだ Entry の詳細の表示。
export function InfoPanel({ libraryId, selection }: InfoPanelProps) {
    const client = useQueryClient();
    const items = [...selection.items.values()];
    const [only] = items;
    // 大きめのサムネイルは、1 件を選んでいて、選択が 200 ms 変わらなかったときにだけ要求する。
    const settled = useSettled(items.length === 1 ? only?.entry : undefined);
    // サムネイル枠の高さと幅。要求する大きさは段階で決め、幅の変化は落ち着いてから反映するので、ドラッグ中に要求し直さない。
    const height = usePanelWidth(INFO_THUMBNAIL_HEIGHT_KEY, INFO_THUMBNAIL_HEIGHT);
    const [frameWidth, setFrameWidth] = useState(0);
    const settledWidth = useSettled(frameWidth) ?? frameWidth;
    const side = infoThumbnailSide(settledWidth, useSettled(height.width) ?? height.width);
    const { wants } = useImageWants(libraryId, 1, false, "fit", side);
    useEffect(() => {
        wants.setWanted(settled !== undefined && isThumbnailable(settled) ? new Map([[settled.path, 0]]) : new Map());
    }, [wants, settled]);
    const thumbnail = useSyncExternalStore(wants.subscribe, () =>
        settled === undefined ? IDLE : wants.getState(settled.path),
    );
    // メディア情報も同じ規則で、画像と動画だけを要求する。
    const mediaTarget = settled !== undefined && hasMediaInfo(settled) ? settled : undefined;
    const media = useQuery({
        ...mediaInfoQuery(libraryId, mediaTarget?.path ?? ""),
        enabled: mediaTarget !== undefined,
    });
    if (only === undefined) {
        return <p className="p-4 text-sm text-muted-foreground">{t("info.chooseEntry")}</p>;
    }
    if (items.length > 1) {
        return <SelectionSummary count={items.length} bytes={selectionBytes(selection)} />;
    }
    return (
        <EntryDetail
            entry={only.entry}
            onRetry={() => retryEntryDetails(client, libraryId, [only.entry.path])}
            thumbnailHeight={height}
            onFrameWidth={setFrameWidth}
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
            <h2 className="font-heading text-sm font-semibold">{t("info.selectedCount", { count })}</h2>
            <p className="text-xs text-muted-foreground">{t("info.total", { bytes: formatBytes(bytes) })}</p>
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
    thumbnailHeight,
    onFrameWidth,
    libraryId,
    tagsTarget,
    media,
}: {
    thumbnailHeight: ReturnType<typeof usePanelWidth>;
    onFrameWidth(width: number): void;
    entry: Entry;
    libraryId: string;
    /// 評価とタグを出す Entry。選択が落ち着いた画像と動画だけで、メディア情報と同じ規則で決める。
    tagsTarget: Entry | undefined;
    onRetry(): void;
    thumbnailUrl: string | undefined;
    media: MediaState | undefined;
}) {
    const frameRef = useRef<HTMLDivElement>(null);
    useEffect(() => {
        const frame = frameRef.current;
        if (!frame) return;
        const observer = new ResizeObserver(() => onFrameWidth(frame.clientWidth));
        observer.observe(frame);
        return () => observer.disconnect();
    }, [onFrameWidth]);
    const kind = entryKind(entry);
    const hasSize = kind !== "folder" && kind !== "archive";
    // タイムラインの Entry は Library を横断するので、Library の名前と、Library 相対パスを分けて出す。
    const address = entryAddress(libraryId, entry.path);
    const libraries = useQuery({ ...librariesQuery, enabled: libraryId === CROSS_LIBRARY_SCOPE });
    const libraryName = libraries.data?.find((library) => library.id === address.libraryId)?.name ?? address.libraryId;
    return (
        <>
            <div className="p-4">
                <div
                    ref={frameRef}
                    className="flex items-center justify-center overflow-hidden rounded-lg bg-muted text-muted-foreground"
                    style={{ height: thumbnailHeight.width }}
                >
                    {thumbnailUrl === undefined ? (
                        <EntryIcon kind={kind} className="size-16" />
                    ) : (
                        <img src={thumbnailUrl} alt="" draggable={false} className="h-full w-full object-contain" />
                    )}
                </div>
                <ResizeHandle
                    label={t("info.thumbnailHeight")}
                    range={INFO_THUMBNAIL_HEIGHT}
                    width={thumbnailHeight.width}
                    orientation="horizontal"
                    grows="down"
                    onResize={thumbnailHeight.resize}
                    onCommit={thumbnailHeight.commit}
                />
                <h2 className="mt-3 truncate font-heading text-sm font-semibold" title={entry.name}>
                    {entry.name}
                </h2>
            </div>
            <Separator />
            <dl className="px-4 py-3">
                <InfoRow label={t("info.kind")} value={ENTRY_KIND_LABELS[kind]} />
                {hasSize && <InfoRow label={t("info.size")} value={formatBytes(entry.metadata?.size)} />}
                <InfoRow label={t("info.modified")} value={formatModifiedAt(entry.metadata?.modifiedAt)} />
                {libraryId === CROSS_LIBRARY_SCOPE && <InfoRow label="Library" value={libraryName} />}
                <InfoRow label={t("info.place")} value={address.path} />
            </dl>
            {entry.detailError && (
                <p role="alert" className="px-4 pb-3 text-xs text-muted-foreground">
                    {t("info.detailFailed")}
                    <Button variant="link" onClick={onRetry}>
                        {t("common.retry")}
                    </Button>
                </p>
            )}
            {media?.isPending && <p className="px-4 pb-3 text-xs text-muted-foreground">{t("common.loading")}</p>}
            {media?.isError && (
                <p role="alert" className="px-4 pb-3 text-xs text-muted-foreground">
                    {t("info.mediaFailed")}
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
                    <InfoRow label={t("info.duration")} value={formatDuration(video.durationSecs)} />
                    <InfoRow label={t("info.resolution")} value={formatDimensions(video.width, video.height)} />
                    <InfoRow
                        label={t("info.codec")}
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
            [t("info.takenAt"), shooting.takenAt === undefined ? undefined : formatTakenAt(shooting.takenAt)],
            [t("info.camera"), formatCamera(shooting.cameraMake, shooting.cameraModel)],
            [t("info.lens"), shooting.lensModel],
            [
                t("info.focalLength"),
                shooting.focalLengthMm === undefined ? undefined : formatFocalLength(shooting.focalLengthMm),
            ],
            [t("info.aperture"), shooting.fNumber === undefined ? undefined : formatFNumber(shooting.fNumber)],
            [
                t("info.exposure"),
                shooting.exposureTimeSecs === undefined ? undefined : formatExposureTime(shooting.exposureTimeSecs),
            ],
            [t("info.iso"), shooting.iso === undefined ? undefined : String(shooting.iso)],
        ];
        const present = rows.filter((row): row is [string, string] => row[1] !== undefined);
        return (
            <>
                <Separator />
                <dl className="px-4 py-3">
                    <InfoRow
                        label={t("info.dimensions")}
                        value={formatDimensions(image.width, image.height, image.rotation)}
                    />
                </dl>
                {present.length > 0 && (
                    <>
                        <Separator />
                        <div className="px-4 py-3">
                            <h3 className="mb-1 text-xs font-medium">{t("info.shooting")}</h3>
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
