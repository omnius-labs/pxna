import { expect, it } from "vitest";
import { fileContentUrl, originalImageUrl } from "./urls";

it("Library とパスをエンコードして単一ファイルの content URL を作る", () => {
    expect(fileContentUrl("ops", "/pack.zip")).toBe("/api/v1/libraries/ops/files/content?path=%2Fpack.zip");
    expect(fileContentUrl("lib id", "/日本語/a b.txt")).toBe(
        "/api/v1/libraries/lib%20id/files/content?path=%2F%E6%97%A5%E6%9C%AC%E8%AA%9E%2Fa+b.txt",
    );
    expect(originalImageUrl("ops", "/pack.zip")).toBe(fileContentUrl("ops", "/pack.zip"));
});
