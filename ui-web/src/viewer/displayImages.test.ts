import { expect, it } from "vitest";

import type { ImageSpec } from "../images/spec";
import { FakeImageSource } from "../testing/images";
import { DisplayImages } from "./displayImages";

const spec = (path: string): ImageSpec => ({ libraryId: "photos", path, kind: "display", width: 2048, height: 1536 });

function setup() {
    const source = new FakeImageSource();
    return { source, images: new DisplayImages(source) };
}

/// 欲しいと言われた画像の、パスと急ぎ度。
const classes = (source: FakeImageSource) =>
    Object.fromEntries(source.wanted().map((want) => [want.spec.path, want.class]));

it("wants the shown image as interactive and the neighbors as prefetch", async () => {
    const { source, images } = setup();
    images.prefetch(spec("/next"));
    const shown = images.show(spec("/a"));
    expect(classes(source)).toEqual({ "/next": "prefetch", "/a": "interactive" });

    source.ready(spec("/a"), "blob:a");
    await expect(shown).resolves.toBe("blob:a");
});

it("raises an image that is still being prefetched to interactive, in the same item", async () => {
    const { source, images } = setup();
    images.prefetch(spec("/a"));
    expect(classes(source)).toEqual({ "/a": "prefetch" });
    const shown = images.show(spec("/a"));

    // The image is wanted once, with a higher class. Nothing is requested again.
    expect(source.wanted()).toHaveLength(1);
    expect(classes(source)).toEqual({ "/a": "interactive" });
    source.ready(spec("/a"), "blob:a");
    await expect(shown).resolves.toBe("blob:a");
});

it("does not lower a shown image to prefetch when it is prefetched again", () => {
    const { source, images } = setup();
    void images.show(spec("/a")).catch(() => undefined);
    images.prefetch(spec("/a"));
    expect(classes(source)).toEqual({ "/a": "interactive" });
});

it("shows a loaded prefetch at once", async () => {
    const { source, images } = setup();
    images.prefetch(spec("/a"));
    source.ready(spec("/a"), "blob:a");
    await expect(images.show(spec("/a"))).resolves.toBe("blob:a");
});

it("rejects a shown image that could not be made, and stays quiet about a prefetch that could not", async () => {
    const { source, images } = setup();
    images.prefetch(spec("/next"));
    source.setState(spec("/next"), { status: "failed", kind: "NotFound" });
    const shown = images.show(spec("/a"));
    source.setState(spec("/a"), { status: "failed", kind: "Timeout" });
    await expect(shown).rejects.toThrow();
});

it("stops wanting the images it no longer keeps, and keeps the most recent loaded ones up to six", () => {
    const { source, images } = setup();
    for (let index = 0; index < 9; index++) {
        images.prefetch(spec(`/${index}`));
        source.ready(spec(`/${index}`), `blob:${index}`);
        // The viewer retains what it shows and the neighbors after each move.
        images.retain([spec(`/${index}`)]);
    }
    images.prefetch(spec("/loading"));
    images.retain([spec("/8")]);

    const wanted = source.wanted().map((want) => want.spec.path);
    // The shown one, and the six loaded images before it. The image still loading and the oldest loaded ones are let go.
    expect(wanted.sort()).toEqual(["/2", "/3", "/4", "/5", "/6", "/7", "/8"]);
    expect(source.wanted().find((want) => want.spec.path === "/8")?.order).toBeLessThan(
        source.wanted().find((want) => want.spec.path === "/7")?.order ?? 0,
    );
});

it("gives everything back on dispose and rejects a show that is still waiting", async () => {
    const { source, images } = setup();
    const shown = images.show(spec("/a"));
    const rejected = expect(shown).rejects.toThrow();
    images.dispose();
    await rejected;
    expect(source.wanted()).toEqual([]);
});
