import type { Entry } from "../library/entries";

/// テスト用の Entry。種類は、フォルダでなければファイルで、メタデータは上書きできる。
export function testEntry(
    path: string,
    overrides: { folder?: boolean; metadata?: Partial<Entry["metadata"]> } = {},
): Entry {
    return {
        path,
        name: path.split("/").at(-1) ?? path,
        kind: overrides.folder ? "folder" : "file",
        metadata: { size: 1, readonly: false, ...overrides.metadata },
        ...(overrides.folder ? {} : { file: { archive: false } }),
    };
}
