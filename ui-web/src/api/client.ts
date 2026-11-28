import createClient from "openapi-fetch";

import type { components, paths } from "./schema";

export type ErrorKind = components["schemas"]["ErrorKind"];

/// API が 2xx 以外を返したときの失敗。`kind` は本文の `Problem` の `code` から取り、本文がなければ持たない。
export class ApiError extends Error {
    readonly status: number;
    readonly kind: ErrorKind | undefined;

    constructor(status: number, kind: ErrorKind | undefined, message: string | undefined) {
        super(message ?? `request failed with status ${status}`);
        this.name = "ApiError";
        this.status = status;
        this.kind = kind;
    }

    /// 認証の要否は HTTP ステータスで判定する。
    get unauthorized(): boolean {
        return this.status === 401;
    }

    get timedOut(): boolean {
        return this.status === 504;
    }
}

export const api = createClient<paths>({ baseUrl: "/api/v1" });

interface FetchResult<T> {
    data?: T;
    error?: unknown;
    response: Response;
}

/// openapi-fetch の結果から本文を取り出し、失敗なら `ApiError` を投げる。
export function unwrap<T>(result: FetchResult<T>): T {
    if (result.response.ok) {
        return result.data as T;
    }
    throw problemError(result.response.status, result.error);
}

/// 失敗した応答の本文（契約の `Problem`）から `ApiError` を作る。本文が読めなければ status だけを持つ。
export function problemError(status: number, body: unknown): ApiError {
    const problem = body as Partial<components["schemas"]["Problem"]> | null | undefined;
    return new ApiError(status, problem?.code, problem?.detail ?? undefined);
}
