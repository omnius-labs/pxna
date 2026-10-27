import { t } from "i18next";

import { currentLanguage } from "../i18n";

const UNITS = ["B", "KB", "MB", "GB", "TB"] as const;

/// バイト数を、1024 を単位に小数 1 桁で表す。
export function formatBytes(bytes: number | undefined): string {
    if (bytes === undefined) return t("format.unknown");
    let value = bytes;
    let unit = 0;
    while (value >= 1024 && unit < UNITS.length - 1) {
        value /= 1024;
        unit += 1;
    }
    return unit === 0 ? `${value} ${UNITS[0]}` : `${value.toFixed(1)} ${UNITS[unit]}`;
}

/// 日時のない Entry でも列の幅と行の高さを保つ。
export function formatModifiedAt(value: string | null | undefined): string {
    if (value == null) return "—";
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "—";
    return new Intl.DateTimeFormat(currentLanguage(), {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
    }).format(date);
}

/// 寸法を「幅 × 高さ」で表す。`rotation` が 90 度か 270 度のときは、表示の向きに合わせて、幅と高さを入れ替える。
export function formatDimensions(width: number, height: number, rotation = 0): string {
    return rotation === 90 || rotation === 270 ? `${height} × ${width}` : `${width} × ${height}`;
}

/// 秒数を「分:秒」か「時:分:秒」で表す。小数の秒は四捨五入する。
export function formatDuration(secs: number): string {
    const total = Math.max(0, Math.round(secs));
    const hours = Math.floor(total / 3600);
    const minutes = Math.floor((total % 3600) / 60);
    const seconds = String(total % 60).padStart(2, "0");
    return hours > 0 ? `${hours}:${String(minutes).padStart(2, "0")}:${seconds}` : `${minutes}:${seconds}`;
}

/// 露出時間を表す。1 秒未満は「1/250 s」の分数にし、1 秒以上は秒数にする。
export function formatExposureTime(secs: number): string {
    return secs > 0 && secs < 1
        ? t("format.exposureFraction", { denominator: Math.round(1 / secs) })
        : t("format.exposureSeconds", { seconds: Number(secs.toFixed(1)) });
}

/// 絞り値を「f/2.8」で表す。
export function formatFNumber(value: number): string {
    return `f/${Number(value.toFixed(1))}`;
}

/// 焦点距離を「24 mm」で表す。
export function formatFocalLength(mm: number): string {
    return `${Number(mm.toFixed(1))} mm`;
}

/// タイムゾーンのない撮影日時（`YYYY-MM-DDTHH:MM:SS`）を、「YYYY-MM-DD HH:MM」で表す。形が違えば、そのまま返す。
export function formatTakenAt(value: string): string {
    const match = /^(\d{4}-\d{2}-\d{2})T(\d{2}:\d{2})/.exec(value);
    return match ? `${match[1]} ${match[2]}` : value;
}

/// カメラのメーカーとモデルを 1 つにまとめる。モデルがメーカー名の最初の語で始まるときは、メーカー名を重ねない。
/// メーカー名は「NIKON CORPORATION」のように会社名を含み、モデル名が「NIKON Z 6」のようにブランド名で始まることがあるためである。
export function formatCamera(make: string | undefined, model: string | undefined): string | undefined {
    if (make === undefined || model === undefined) return model ?? make;
    const brand = make.trim().split(/\s+/)[0] ?? "";
    return brand === "" || model.toLowerCase().startsWith(brand.toLowerCase()) ? model : `${make} ${model}`;
}
