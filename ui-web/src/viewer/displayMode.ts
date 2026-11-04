import { useState } from "react";

/// The display mode saved per device in `localStorage`. The value is one of `ViewerDisplayMode`.
export const VIEWER_MODE_KEY = "pxna.viewerMode";

export const VIEWER_DISPLAY_MODES = ["standard", "gallery", "focus"] as const;

export type ViewerDisplayMode = (typeof VIEWER_DISPLAY_MODES)[number];

/// Reads the saved mode. Without a saved value, or with one that cannot be read, the viewer opens in standard mode.
export function readViewerDisplayMode(): ViewerDisplayMode {
    try {
        const saved = window.localStorage.getItem(VIEWER_MODE_KEY);
        return VIEWER_DISPLAY_MODES.find((mode) => mode === saved) ?? "standard";
    } catch {
        return "standard";
    }
}

/// Saves the mode. A failure does not stop viewing, so it throws nothing.
export function writeViewerDisplayMode(mode: ViewerDisplayMode): void {
    try {
        window.localStorage.setItem(VIEWER_MODE_KEY, mode);
    } catch {
        // Without saving, the mode lasts only while the viewer is open.
    }
}

/// The saved display mode and its setter, which also saves it.
export function useViewerDisplayMode() {
    const [mode, setMode] = useState(readViewerDisplayMode);
    const change = (next: ViewerDisplayMode) => {
        setMode(next);
        writeViewerDisplayMode(next);
    };
    return [mode, change] as const;
}
