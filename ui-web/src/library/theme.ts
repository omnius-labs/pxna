const DARK_QUERY = "(prefers-color-scheme: dark)";

/// OS のライトとダークの設定に従って、`<html>` の `dark` クラスを付け外しする。設定が変わったら追従する。
/// 返す関数で追従をやめる。
export function followSystemTheme(
    root: HTMLElement = document.documentElement,
    media: MediaQueryList = window.matchMedia(DARK_QUERY),
): () => void {
    const apply = () => root.classList.toggle("dark", media.matches);
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
}
