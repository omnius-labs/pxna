import { useQuery } from "@tanstack/react-query";
import { useEffect, useRef, useState } from "react";

import { mediaInfoQuery } from "../library/queries";
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
            if (!video) throw new Error("動画の情報を取得できませんでした。");
            return video;
        },
    });
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
                video.src = originalImageUrl(libraryId, path);
                return;
            }
            const { default: Hls } = await import("hls.js");
            if (closed) return;
            const mode = playbackMode(info.contentType, (type) => video.canPlayType(type), Hls.isSupported());
            if (mode === "unsupported") throw new Error("このブラウザは動画再生に対応していません。");
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
                        setError("動画を再生できませんでした。閉じて、もう一度開いてください。");
                    }
                });
                hls.loadSource(session.playlistUrl);
                hls.attachMedia(video);
            }
        };
        void start().catch((cause: unknown) => {
            if (!closed) {
                release?.();
                setError(cause instanceof Error ? cause.message : "動画を読み込めませんでした。");
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
            {media.isPending ? <p className="absolute text-neutral-300">読み込み中…</p> : null}
            {media.isError || error ? (
                <p role="alert" className="absolute z-10 rounded bg-black/80 p-4 text-neutral-200">
                    {error ?? "動画情報を読み込めませんでした。"}
                </p>
            ) : null}
            {/* biome-ignore lint/a11y/useMediaCaption: 字幕トラックのない動画ソースを扱う。提供開始時に track を追加する */}
            <video
                ref={ref}
                controls
                autoPlay
                playsInline
                tabIndex={0}
                aria-label="動画プレイヤー"
                className="h-full max-h-full w-full object-contain"
                onEnded={() => done.current?.()}
                onError={() => setError("動画を再生できませんでした。閉じて、もう一度開いてください。")}
            />
        </div>
    );
}
