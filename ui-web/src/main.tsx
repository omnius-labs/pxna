import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { Fragment, StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { clientTimings } from "./api/clientTimings";
import { createAppQueryClient, leaveForLogin } from "./auth/unauthorized";
import { Toaster } from "./components/ui/sonner";
import { useLanguage } from "./i18n";
import "./index.css";
import { notifyImageSessionEnded, onImageSessionEnded, setImageSource } from "./images/source";
import { followSystemTheme } from "./library/theme";
import { createAppRouter } from "./router";
import { createBrowserImageSocket } from "./sockets/image";

followSystemTheme();

const navigator = () => ({
    currentHref: () => router.state.location.href,
    currentPathname: () => router.state.location.pathname,
    toLogin: (redirect: string) => void router.navigate({ to: "/login", search: { redirect }, replace: true }),
});
const queryClient = createAppQueryClient(navigator);
const router = createAppRouter(queryClient);
clientTimings.attach(queryClient);
// image socket が、セッションが終わったと言って閉じられたら、API の 401 と同じようにログインへ移す。
onImageSessionEnded(() => leaveForLogin(queryClient, navigator()));
setImageSource(createBrowserImageSocket(notifyImageSessionEnded));

const root = document.getElementById("root");
if (!root) {
    throw new Error("missing #root element");
}

/// 文字列は描画のたびに引くので、言語が変わったら画面全体を作り直す。表示中の画面の状態（開いたメニューなど）は失うが、サーバの状態は `QueryClient` に残る。
function App() {
    const language = useLanguage();
    return (
        <Fragment key={language}>
            <QueryClientProvider client={queryClient}>
                <RouterProvider router={router} />
            </QueryClientProvider>
            <Toaster />
        </Fragment>
    );
}

createRoot(root).render(
    <StrictMode>
        <App />
    </StrictMode>,
);
