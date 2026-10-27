import { type QueryClient, queryOptions } from "@tanstack/react-query";

import { api, unwrap } from "../api/client";
import { imageSource } from "../images/source";

export const currentUserQuery = queryOptions({
    queryKey: ["auth", "me"],
    queryFn: async () => unwrap(await api.GET("/auth/me")),
    retry: false,
    staleTime: Infinity,
});

export async function fetchSetupInitialized(): Promise<boolean> {
    return unwrap(await api.GET("/setup/status")).initialized;
}

export interface Credentials {
    username: string;
    password: string;
}

export async function login(credentials: Credentials): Promise<void> {
    unwrap(await api.POST("/auth/login", { body: { ...credentials, sessionTransport: "cookie" } }));
}

export async function createOwner(credentials: Credentials): Promise<void> {
    unwrap(await api.POST("/setup", { body: { ...credentials, sessionTransport: "cookie" } }));
}

/// サーバ側のセッションを失効させてから、手元のキャッシュを捨てる。受け取った画像は、ログアウトの後に同じブラウザを使う人に
/// Library の中身を見せてしまうので、IndexedDB に残した分も消す。
export async function logout(queryClient: QueryClient): Promise<void> {
    unwrap(await api.DELETE("/auth/session"));
    queryClient.clear();
    await imageSource().clear();
}
