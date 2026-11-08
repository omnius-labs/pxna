import path from "node:path";

import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

// The daemon's default listen address (listen_addr in daemon/config/pxna.toml.example).
// Set PXNA_DAEMON_ORIGIN to reach another daemon.
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
            // Do not rewrite Host. The daemon compares Origin with Host, so rewriting Host to the
            // daemon's address makes cookie-bearing mutations answer 403. `ws` also forwards the
            // image socket's WebSocket upgrade to the same target.
            "/api": { target: daemonOrigin, changeOrigin: false, ws: true },
        },
    },
    test: {
        environment: "node",
        include: ["src/**/*.test.ts", "src/**/*.test.tsx"],
        // Every test fakes the image source so it never opens a real WebSocket or IndexedDB.
        setupFiles: ["./src/testing/setup.ts"],
    },
});
