import { t } from "i18next";

import { ApiError } from "../api/client";

/// 失敗を、利用者に見せる文言にする。API の失敗は `kind` から今の言語の文言を引く。
/// daemon の `message` は英語のログ向けなので、`kind` がないときだけ使う。
export function describeError(error: unknown, fallback: string): string {
    if (error instanceof ApiError && error.kind !== undefined) return t(`apiErrors.${error.kind}`);
    return error instanceof Error ? error.message : fallback;
}
