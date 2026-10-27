import { useQuery } from "@tanstack/react-query";
import { t } from "i18next";
import { useEffect, useRef, useState } from "react";
import { clientTimings } from "../api/clientTimings";
import { describeError } from "../i18n/errors";
import { mediaInfoQuery } from "../library/queries";
import { entryAddress } from "../location";
import { originalImageUrl } from "../viewer/urls";
import { acquireVideoSession, playbackMode } from "../viewer/video";

interface VideoPlayerProps {
    libraryId: string;
    path: string;
    /// 最後まで再生したか、再生できないと分かったときに呼ぶ。スライドショーが次へ進むのに使う。
    onDone?: () => void;
}

/** クライアントの再生能力に応じて元 file か HLS を選び、閉じると再生資源を回収する。 */
export function VideoPlayer({ libraryId, path, onDone }: VideoPlayerProps) {
    const ref = useRef<HTMLVideoElement>(null);
    const [error, setError] = useState<string>();
    const media = useQuery({
        ...mediaInfoQuery(libraryId, path),
        select: ({ video }) => {
            if (!video) throw new Error(t("video.infoFailed"));
            return video;
        },
    });
    // 開いてから最初のフレームが出るまでを測る。動画ごとに 1 回だけ記録し、開始前に閉じたり失敗したりした分は記録しない。
    const timing = useRef<{ startedAt: number; mode?: "direct" | "native-hls" | "hls-js"; recorded: boolean }>({
        startedAt: performance.now(),
        recorded: false,
    });
    // biome-ignore lint/correctness/useExhaustiveDependencies: 動画が変わったときだけ測り直す
    useEffect(() => {
        timing.current = { startedAt: performance.now(), recorded: false };
    }, [libraryId, path]);
    const done = useRef(onDone);
    useEffect(() => {
        done.current = onDone;
    });
    // 再生できないと分かったときも、スライドショーを止めずに次へ進める。
    const failed = error !== undefined || media.isError;
    useEffect(() => {
        if (failed) done.current?.();
    }, [failed]);
    useEffect(() => {
        const video = ref.current;
        const info = media.data;
        if (!video || !info) return;
        let closed = false;
        let release: (() => void) | undefined;
        let destroy: (() => void) | undefined;
        const start = async () => {
            if (video.canPlayType(info.contentType)) {
                timing.current.mode = "direct";
                video.src = originalImageUrl(libraryId, path);
                return;
            }
            const { default: Hls } = await import("hls.js");
            if (closed) return;
            const mode = playbackMode(info.contentType, (type) => video.canPlayType(type), Hls.isSupported());
            if (mode === "unsupported") throw new Error(t("video.unsupported"));
            timing.current.mode = mode === "native-hls" ? "native-hls" : "hls-js";
            const resource = acquireVideoSession(libraryId, path);
            release = resource.release;
            const session = await resource.session;
            if (closed) return;
            if (mode === "native-hls") {
                video.src = session.playlistUrl;
            } else {
                const hls = new Hls();
                destroy = () => hls.destroy();
                hls.on(Hls.Events.ERROR, (_event, data) => {
                    if (data.fatal && !closed) {
                        hls.destroy();
                        release?.();
                        setError(t("video.playFailed"));
                    }
                });
                hls.loadSource(session.playlistUrl);
                hls.attachMedia(video);
            }
        };
        void start().catch((cause: unknown) => {
            if (!closed) {
                release?.();
                setError(describeError(cause, t("video.loadFailed")));
            }
        });
        return () => {
            closed = true;
            video.pause();
            destroy?.();
            video.removeAttribute("src");
            video.load();
            release?.();
        };
    }, [libraryId, path, media.data]);

    return (
        <div className="flex h-full items-center justify-center p-4">
            {media.isPending ? <p className="absolute text-neutral-300">{t("common.loading")}</p> : null}
            {media.isError || error ? (
                <p role="alert" className="absolute z-10 rounded bg-black/80 p-4 text-neutral-200">
                    {error ?? t("video.infoLoadFailed")}
                </p>
            ) : null}
            {/* biome-ignore lint/a11y/useMediaCaption: 字幕トラックのない動画ソースを扱う。提供開始時に track を追加する */}
            <video
                ref={ref}
                controls
                autoPlay
                playsInline
                tabIndex={0}
                aria-label={t("video.player")}
                className="h-full max-h-full w-full object-contain"
                onPlaying={() => {
                    const { startedAt, mode, recorded } = timing.current;
                    if (recorded) return;
                    timing.current.recorded = true;
                    clientTimings.record({
                        ...entryAddress(libraryId, path),
                        operation: "video_start",
                        durationMs: Math.round((performance.now() - startedAt) * 10) / 10,
                        mode,
                    });
                }}
                onEnded={() => done.current?.()}
                onError={() => setError(t("video.playFailed"))}
            />
        </div>
    );
}
