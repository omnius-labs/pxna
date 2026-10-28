import { expect, it } from "vitest";

import type { FrameHeader } from "../api/imageSocket";
import { decodeFrame, encodeFrame, parseServerMessage } from "./protocol";

const header: FrameHeader = {
    id: 5,
    etag: "a".repeat(64),
    contentType: "image/webp",
    frameCount: 8,
    offset: 65536,
    total: 100000,
};

it("reads a frame back into its header and chunk", () => {
    const frame = encodeFrame(header, new Uint8Array([1, 2, 3]));
    const decoded = decodeFrame(frame);
    expect(decoded?.header).toEqual(header);
    expect([...(decoded?.chunk ?? [])]).toEqual([1, 2, 3]);
});

it("reads the length as 4 bytes, big-endian, as the daemon writes it", () => {
    const frame = new Uint8Array(encodeFrame(header, new Uint8Array(0)));
    const length = JSON.stringify(header).length;
    expect([...frame.slice(0, 4)]).toEqual([0, 0, length >> 8, length & 0xff]);
});

it("refuses a frame that is shorter than its length says, or whose header is not JSON", () => {
    expect(decodeFrame(new ArrayBuffer(2))).toBeUndefined();
    const short = encodeFrame(header, new Uint8Array(0));
    expect(decodeFrame(short.slice(0, 10))).toBeUndefined();
    const broken = new Uint8Array([0, 0, 0, 3, 110, 111, 112]);
    expect(decodeFrame(broken.buffer)).toBeUndefined();
});

it("reads the two messages the daemon sends and ignores anything else", () => {
    expect(parseServerMessage('{"type":"unchanged","id":4}')).toEqual({ type: "unchanged", id: 4 });
    expect(parseServerMessage('{"type":"failed","id":4,"kind":"NotFound"}')).toMatchObject({
        type: "failed",
        kind: "NotFound",
    });
    expect(parseServerMessage('{"type":"other"}')).toBeUndefined();
    expect(parseServerMessage("nonsense")).toBeUndefined();
});
