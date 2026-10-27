import { beforeEach } from "vitest";

import { setImageSource } from "../images/source";
import { FakeImageSource } from "./images";

// 画像の窓口は、テストごとに新しい偽物にする。画像を使うテストは、必要なら `install()` で自分の偽物に差し替える。
beforeEach(() => {
    setImageSource(new FakeImageSource());
});
