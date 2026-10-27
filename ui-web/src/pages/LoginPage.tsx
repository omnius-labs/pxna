import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { t } from "i18next";
import { useEffect } from "react";

import { ApiError } from "../api/client";
import { currentUserQuery, fetchSetupInitialized, login } from "../auth/session";
import { CredentialsForm } from "../components/CredentialsForm";
import { safeRedirect } from "../location";

export function LoginPage() {
    const { redirect } = useSearch({ from: "/login" });
    const navigate = useNavigate();
    const queryClient = useQueryClient();
    const initialized = useQuery({ queryKey: ["setup", "status"], queryFn: fetchSetupInitialized });

    // オーナーが未登録なら、ログインではなく初期セットアップへ移す。
    useEffect(() => {
        if (initialized.data === false) {
            void navigate({ to: "/setup", search: redirect === undefined ? {} : { redirect }, replace: true });
        }
    }, [initialized.data, navigate, redirect]);

    const signIn = useMutation({
        mutationFn: login,
        onSuccess: async () => {
            await queryClient.invalidateQueries({ queryKey: currentUserQuery.queryKey });
            await navigate({ href: safeRedirect(redirect), replace: true });
        },
    });

    return (
        <CredentialsForm
            title={t("pages.login")}
            submitLabel={t("pages.login")}
            passwordAutoComplete="current-password"
            error={signIn.error ? loginErrorMessage(signIn.error) : undefined}
            pending={signIn.isPending}
            onSubmit={(credentials) => signIn.mutate(credentials)}
        />
    );
}

function loginErrorMessage(error: Error): string {
    if (error instanceof ApiError && error.unauthorized) {
        return t("pages.loginInvalid");
    }
    return t("pages.loginFailed");
}
