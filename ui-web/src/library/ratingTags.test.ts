import { QueryClient } from "@tanstack/react-query";
import { afterEach, expect, it, vi } from "vitest";

import { normalizeTag, putEntryTags, putRating, ratingQuery, tagVocabularyQuery } from "./ratingTags";

const mocks = vi.hoisted(() => ({ put: vi.fn() }));
vi.mock("../api/client", async (importOriginal) => ({
    ...(await importOriginal<typeof import("../api/client")>()),
    api: { PUT: mocks.put },
}));
afterEach(() => mocks.put.mockReset());

const failure = { error: { kind: "NotFound", message: "x" }, response: new Response(null, { status: 404 }) };

it("評価は保存の前にキャッシュへ入れ、失敗したら元の値へ戻して例外を投げる", async () => {
    const client = new QueryClient();
    const key = ratingQuery("photos", "/a.png").queryKey;
    client.setQueryData(key, "good");
    let seen: unknown;
    mocks.put.mockImplementation(async () => {
        seen = client.getQueryData(key);
        return failure;
    });
    await expect(putRating(client, "photos", "/a.png", "bad")).rejects.toThrow();
    expect(seen).toBe("bad");
    expect(client.getQueryData(key)).toBe("good");
});

it("読み込み前の評価の保存が失敗したら、キャッシュを空に戻す", async () => {
    const client = new QueryClient();
    mocks.put.mockResolvedValue(failure);
    await expect(putRating(client, "photos", "/a.png", "good")).rejects.toThrow();
    expect(client.getQueryData(ratingQuery("photos", "/a.png").queryKey)).toBeUndefined();
});

it("タグの保存の結果を、Entry のキャッシュと語彙へ入れる", async () => {
    const client = new QueryClient();
    client.setQueryData(tagVocabularyQuery.queryKey, ["dog"]);
    mocks.put.mockResolvedValue({ data: { tags: ["cat", "trip"] }, response: new Response() });
    expect(await putEntryTags(client, "photos", "/a.png", ["trip", "cat"])).toEqual(["cat", "trip"]);
    expect(client.getQueryData(["entry-tags", "photos", "/a.png"])).toEqual(["cat", "trip"]);
    expect(client.getQueryData(tagVocabularyQuery.queryKey)).toEqual(["cat", "dog", "trip"]);
});

it("語は前後の空白を除き、空、長すぎる、制御文字を含む語は使えない", () => {
    expect(normalizeTag("  旅行 ")).toBe("旅行");
    expect(normalizeTag("   ")).toBeUndefined();
    expect(normalizeTag("a\nb")).toBeUndefined();
    expect(normalizeTag("あ".repeat(64))).toBe("あ".repeat(64));
    expect(normalizeTag("あ".repeat(65))).toBeUndefined();
});
