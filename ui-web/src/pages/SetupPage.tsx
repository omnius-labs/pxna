import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { useEffect } from "react";

import { ApiError } from "../api/client";
import { createOwner, currentUserQuery, fetchSetupInitialized } from "../auth/session";
import { CredentialsForm } from "../components/CredentialsForm";
import { safeRedirect } from "../location";

export function SetupPage() {
    const { redirect } = useSearch({ from: "/setup" });
    const navigate = useNavigate();
    const queryClient = useQueryClient();
    const initialized = useQuery({ queryKey: ["setup", "status"], queryFn: fetchSetupInitialized });

    const setup = useMutation({
        mutationFn: createOwner,
        onSuccess: async () => {
            queryClient.setQueryData(["setup", "status"], true);
            await queryClient.invalidateQueries({ queryKey: currentUserQuery.queryKey });
            await navigate({ href: safeRedirect(redirect), replace: true });
        },
        onError: async (error) => {
            if (error instanceof ApiError && error.kind === "AlreadyInitialized") {
                await queryClient.invalidateQueries({ queryKey: ["setup", "status"] });
            }
        },
    });

    // オーナーが登録済みなら、初期セットアップではなくログインへ移す。
    // この画面で登録した後は、セッションを持って元の URL へ戻るので移さない。
    const registering = setup.isPending || setup.isSuccess;
    useEffect(() => {
        if (initialized.data === true && !registering) {
            void navigate({ to: "/login", search: redirect === undefined ? {} : { redirect }, replace: true });
        }
    }, [initialized.data, registering, navigate, redirect]);

    return (
        <CredentialsForm
            title="初期セットアップ"
            description="Pxna のオーナーを登録します。オーナーは 1 人だけです。"
            submitLabel="登録する"
            passwordAutoComplete="new-password"
            error={setup.error ? setupErrorMessage(setup.error) : undefined}
            pending={setup.isPending}
            onSubmit={(credentials) => setup.mutate(credentials)}
        />
    );
}

function setupErrorMessage(error: Error): string {
    if (error instanceof ApiError && error.kind === "InvalidRequest") {
        return "ユーザ名かパスワードの形式が正しくありません。";
    }
    return "登録できませんでした。";
}
