import { t } from "i18next";
import { Image as ImageIcon, Settings } from "lucide-react";
import type { ReactNode } from "react";

import type { DisplaySet } from "../library/preferences";
import { MAX_FILMSTRIP_SIZE, MIN_FILMSTRIP_SIZE } from "../viewer/filmstrip";
import { Popover, PopoverContent, PopoverTrigger } from "./ui/popover";
import { Slider } from "./ui/slider";
import { ToggleGroup, ToggleGroupItem } from "./ui/toggle-group";

export interface ViewerSettingsProps {
    /// Whether the filmstrip size is shown. Only the wide screen has a filmstrip.
    filmstrip: boolean;
    /// Whether the original image swap is shown. Only an image is swapped.
    originalImage: boolean;
    set: Pick<DisplaySet, "filmstripSize" | "originalImage">;
    onChange: (patch: Partial<DisplaySet>) => void;
    open: boolean;
    onOpenChange: (open: boolean) => void;
    /// Where the popover is drawn. The outer frame element of the modal, so that it stays above the modal and in the fullscreen element.
    container: HTMLElement | null;
}

/// The gear button of the viewer header and the popover of the settings that only the viewer uses.
/// It renders nothing when neither setting applies, so the gear never opens an empty panel.
export function ViewerSettings({
    filmstrip,
    originalImage,
    set,
    onChange,
    open,
    onOpenChange,
    container,
}: ViewerSettingsProps) {
    if (!filmstrip && !originalImage) {
        return null;
    }
    return (
        <Popover open={open} onOpenChange={onOpenChange}>
            <PopoverTrigger asChild>
                <button
                    type="button"
                    aria-label={t("viewerSettings.label")}
                    title={t("viewerSettings.label")}
                    className="rounded p-1.5 hover:bg-white/10"
                >
                    <Settings className="size-4" />
                </button>
            </PopoverTrigger>
            <PopoverContent align="end" container={container} className="w-72 gap-4">
                {filmstrip && (
                    <Field label={t("viewerSettings.filmstripSize")}>
                        <div className="flex items-center gap-2">
                            <ImageIcon className="size-3 shrink-0 text-muted-foreground" />
                            <Slider
                                value={[set.filmstripSize]}
                                min={MIN_FILMSTRIP_SIZE}
                                max={MAX_FILMSTRIP_SIZE}
                                step={1}
                                aria-label={t("viewerSettings.filmstripSize")}
                                onValueChange={([filmstripSize]) =>
                                    filmstripSize !== undefined && onChange({ filmstripSize })
                                }
                            />
                            <ImageIcon className="size-4 shrink-0 text-muted-foreground" />
                        </div>
                    </Field>
                )}
                {originalImage && (
                    <Field label={t("viewerSettings.originalImage")}>
                        <ToggleGroup
                            type="single"
                            variant="outline"
                            size="sm"
                            value={set.originalImage}
                            aria-label={t("viewerSettings.originalImage")}
                            // Pressing the selected item again empties the value, so ignore it.
                            onValueChange={(value) =>
                                (value === "auto" || value === "onZoom") && onChange({ originalImage: value })
                            }
                        >
                            <ToggleGroupItem value="auto">{t("viewerSettings.auto")}</ToggleGroupItem>
                            <ToggleGroupItem value="onZoom">{t("viewerSettings.onZoom")}</ToggleGroupItem>
                        </ToggleGroup>
                    </Field>
                )}
            </PopoverContent>
        </Popover>
    );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
    return (
        <div className="flex flex-col gap-1.5">
            <span className="text-xs font-medium text-muted-foreground">{label}</span>
            {children}
        </div>
    );
}
