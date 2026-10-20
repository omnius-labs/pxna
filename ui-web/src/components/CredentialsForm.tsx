import { type FormEvent, useState } from "react";

import type { Credentials } from "../auth/session";

interface CredentialsFormProps {
    title: string;
    description?: string;
    submitLabel: string;
    passwordAutoComplete: "current-password" | "new-password";
    error: string | undefined;
    pending: boolean;
    onSubmit: (credentials: Credentials) => void;
}

/// 初期セットアップとログインが共有する、ユーザ名とパスワードのフォーム。
export function CredentialsForm({
    title,
    description,
    submitLabel,
    passwordAutoComplete,
    error,
    pending,
    onSubmit,
}: CredentialsFormProps) {
    const [username, setUsername] = useState("");
    const [password, setPassword] = useState("");

    const submit = (event: FormEvent) => {
        event.preventDefault();
        onSubmit({ username, password });
    };

    return (
        <main className="flex min-h-screen items-center justify-center bg-muted">
            <form onSubmit={submit} className="w-80 space-y-4 rounded-lg bg-card p-6 text-card-foreground shadow">
                <h1 className="text-lg font-semibold">{title}</h1>
                {description && <p className="text-sm text-muted-foreground">{description}</p>}
                <label className="block text-sm">
                    ユーザ名
                    <input
                        name="username"
                        className="mt-1 w-full rounded border border-input bg-background px-2 py-1"
                        autoComplete="username"
                        required
                        maxLength={128}
                        pattern="[A-Za-z0-9._\-]+"
                        value={username}
                        onChange={(event) => setUsername(event.target.value)}
                    />
                </label>
                <label className="block text-sm">
                    パスワード
                    <input
                        name="password"
                        type="password"
                        className="mt-1 w-full rounded border border-input bg-background px-2 py-1"
                        autoComplete={passwordAutoComplete}
                        required
                        minLength={8}
                        maxLength={256}
                        value={password}
                        onChange={(event) => setPassword(event.target.value)}
                    />
                </label>
                {error && (
                    <p role="alert" className="text-sm text-destructive">
                        {error}
                    </p>
                )}
                <button
                    type="submit"
                    disabled={pending}
                    className="w-full rounded bg-primary py-1.5 text-primary-foreground disabled:opacity-50"
                >
                    {submitLabel}
                </button>
            </form>
        </main>
    );
}
