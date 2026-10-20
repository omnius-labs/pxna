import { useQueryClient } from "@tanstack/react-query";
import { type ReactNode, useEffect, useState } from "react";
import { refreshLibrary } from "../library/listing";
import { UploadContext, UploadStore, watchUploadsBeforeUnload } from "../library/upload";
export function UploadProvider({ children }: { children: ReactNode }) {
    const client = useQueryClient();
    const [store] = useState(() => new UploadStore((libraryId) => refreshLibrary(client, libraryId)));
    useEffect(() => {
        const stop = watchUploadsBeforeUnload(store);
        return () => {
            stop();
            store.cancelAll();
        };
    }, [store]);
    return <UploadContext.Provider value={store}>{children}</UploadContext.Provider>;
}
