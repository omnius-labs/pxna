import createClient from "openapi-fetch";

import type { components, paths } from "./schema";

export type ErrorKind = components["schemas"]["ErrorKind"];

/// API が 2xx 以外を返したときの失敗。`kind` は本文の `ErrorResponse` から取り、本文がなければ持たない。
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
    const body = result.error as { kind?: ErrorKind; message?: string | null } | undefined;
    throw new ApiError(result.response.status, body?.kind, body?.message ?? undefined);
}
