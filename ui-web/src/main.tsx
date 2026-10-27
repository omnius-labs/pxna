import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { clientTimings } from "./api/clientTimings";
import { createAppQueryClient, leaveForLogin } from "./auth/unauthorized";
import { Toaster } from "./components/ui/sonner";
import "./index.css";
import { createBrowserImageChannel } from "./images/channel";
import { notifyImageSessionEnded, onImageSessionEnded, setImageSource } from "./images/source";
import { followSystemTheme } from "./library/theme";
import { createAppRouter } from "./router";

followSystemTheme();

const navigator = () => ({
    currentHref: () => router.state.location.href,
    currentPathname: () => router.state.location.pathname,
    toLogin: (redirect: string) => void router.navigate({ to: "/login", search: { redirect }, replace: true }),
});
const queryClient = createAppQueryClient(navigator);
const router = createAppRouter(queryClient);
clientTimings.attach(queryClient);
// image channel が、セッションが終わったと言って閉じられたら、API の 401 と同じようにログインへ移す。
onImageSessionEnded(() => leaveForLogin(queryClient, navigator()));
setImageSource(createBrowserImageChannel(notifyImageSessionEnded));

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
