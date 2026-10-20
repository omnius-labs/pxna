import path from "node:path";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// daemon の既定の待ち受けアドレス（daemon/config/pxna.toml.example の listen_addr）。
// 別の daemon へ繋ぐときは、環境変数 `PXNA_DAEMON_ORIGIN` で上書きする。
const daemonOrigin = process.env.PXNA_DAEMON_ORIGIN ?? "http://localhost:6050";

export default defineConfig({
    plugins: [react(), tailwindcss()],
    resolve: {
        alias: {
            "@": path.resolve(import.meta.dirname, "./src"),
        },
    },
    server: {
        proxy: {
            // Host を書き換えない。daemon の Origin の検証は Origin を Host と比べるため、
            // Host を daemon のアドレスへ書き換えると、cookie を持つ変更系の要求が 403 になる。
            "/api": { target: daemonOrigin, changeOrigin: false },
        },
    },
    test: {
        environment: "node",
        include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
    },
});
