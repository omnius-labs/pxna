import { AppShell } from "../components/AppShell";
import { SettingsMenu } from "../components/SettingsMenu";
import { StatusBar } from "../components/StatusBar";

export function HomePage() {
    return (
        <AppShell
            actions={<SettingsMenu />}
            footer={<StatusBar totalCount={undefined} selectedCount={0} selectedBytes={0} />}
        >
            <p className="p-4 text-muted-foreground">左のツリーから Library を選んでください。</p>
        </AppShell>
    );
}
