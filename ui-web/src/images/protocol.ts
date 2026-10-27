import type { FrameHeader, ServerMessage } from "../api/imageChannel";

/// セッションが終わったために daemon が閉じた。ログインへ移る。
export const CLOSE_SESSION_ENDED = 4401;
/// こちらが契約か上限を破った。繰り返しても同じなので、再接続しない。
export const CLOSE_BROKE_CONTRACT = 4400;

/// バイナリのフレーム: 4 バイトのビッグエンディアンの長さ、その長さの JSON のヘッダ、画像の一片。
export function decodeFrame(buffer: ArrayBuffer): { header: FrameHeader; chunk: Uint8Array } | undefined {
    if (buffer.byteLength < 4) return undefined;
    const length = new DataView(buffer).getUint32(0, false);
    if (buffer.byteLength < 4 + length) return undefined;
    try {
        const header = JSON.parse(new TextDecoder().decode(new Uint8Array(buffer, 4, length))) as FrameHeader;
        return { header, chunk: new Uint8Array(buffer, 4 + length) };
    } catch {
        return undefined;
    }
}

/// `decodeFrame` の逆。テストが daemon の代わりにフレームを作るのに使う。
export function encodeFrame(header: FrameHeader, chunk: Uint8Array): ArrayBuffer {
    const json = new TextEncoder().encode(JSON.stringify(header));
    const frame = new Uint8Array(4 + json.length + chunk.length);
    new DataView(frame.buffer).setUint32(0, json.length, false);
    frame.set(json, 4);
    frame.set(chunk, 4 + json.length);
    return frame.buffer;
}

export function parseServerMessage(text: string): ServerMessage | undefined {
    try {
        const message = JSON.parse(text) as ServerMessage;
        return message.type === "unchanged" || message.type === "failed" ? message : undefined;
    } catch {
        return undefined;
    }
}
