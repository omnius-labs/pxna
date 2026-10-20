import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { createAppQueryClient } from "./auth/unauthorized";
import { Toaster } from "./components/ui/sonner";
import "./index.css";
import { followSystemTheme } from "./library/theme";
import { createAppRouter } from "./router";

followSystemTheme();

const queryClient = createAppQueryClient(() => ({
    currentHref: () => router.state.location.href,
    currentPathname: () => router.state.location.pathname,
    toLogin: (redirect) => void router.navigate({ to: "/login", search: { redirect }, replace: true }),
}));
const router = createAppRouter(queryClient);

const root = document.getElementById("root");
if (!root) {
    throw new Error("missing #root element");
}

createRoot(root).render(
    <StrictMode>
        <QueryClientProvider client={queryClient}>
            <RouterProvider router={router} />
        </QueryClientProvider>
        <Toaster />
    </StrictMode>,
);
