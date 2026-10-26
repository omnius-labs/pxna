import { createContext, useContext } from "react";
import type { TransferSource } from "./clipboard";
import type { TransferDestination } from "./collision";
import type { UploadInput } from "./upload";
export interface TransferState {
    busy: boolean;
    canPaste: boolean;
    paste(destination: TransferDestination): Promise<void>;
    upload?(inputs: readonly UploadInput[], destination: TransferDestination): Promise<void>;
    transfer(source: TransferSource, destination: TransferDestination): Promise<void>;
}
export const TransferContext = createContext<TransferState | undefined>(undefined);
export const useTransfer = () => useContext(TransferContext);
