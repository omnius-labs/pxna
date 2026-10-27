import i18next from "i18next";
import { beforeEach } from "vitest";

import "../i18n";
import { setImageSource } from "../images/source";
import { FakeImageSource } from "./images";

// 画像の窓口は、テストごとに新しい偽物にする。画像を使うテストは、必要なら `install()` で自分の偽物に差し替える。
// 文言は英語に固定する。言語の選択はブラウザに残るので、テストごとに消す。
beforeEach(async () => {
    if (typeof window !== "undefined") window.localStorage.clear();
    await i18next.changeLanguage("en");
    setImageSource(new FakeImageSource());
});
