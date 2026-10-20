import { type ReactNode, useState } from "react";
import { ClipboardContext, EntryClipboard } from "../library/clipboard";
export function ClipboardProvider({ children }: { children: ReactNode }) {
    const [store] = useState(() => new EntryClipboard());
    return <ClipboardContext.Provider value={store}>{children}</ClipboardContext.Provider>;
}
