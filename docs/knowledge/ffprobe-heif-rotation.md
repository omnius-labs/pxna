# ffprobe Reports the AVIF irot as the Display Matrix rotation, and the Width and Height Stay as Encoded

**Source: Verified** (ffmpeg 8.1.1 / 2026-10-11)

One item of [knowledge.md](../knowledge.md).

## Fact

With ffmpeg 8.1.1 on macOS (AVIF created with libsvtav1), we confirmed the following about the AVIF `irot` box.

- For a 640×360 AVIF without `irot`, `ffprobe -show_streams` gave a `width` of 640 and a `height` of 360 and had no `side_data_list`.
- When `irot` was added to the same image, `width` and `height` stayed at 640 and 360, `side_data_list` gained a `Display Matrix`, and `rotation` was returned according to the angle of `irot`. For `irot` values 1, 3, and 2, `rotation` was 90, -90, and -180.
- When ffmpeg decoded an AVIF with `irot` to PNG, the default auto-rotation produced an image with the rotation applied. It was 360×640 for `irot` values 1 and 3, and 640×360 for value 2.

We did not check HEIC, the mirroring `imir`, other versions of ffmpeg, or images whose Exif orientation disagrees with `irot`.
HEIC could not be checked because the ffmpeg at hand cannot write HEIC and we have no real file. AVIF and HEIC have `irot` through the same HEIF mechanism, so the same result is expected. This is a conjecture.

## Basis

ffmpeg 8.1.1 did not write `irot` even when creating an AVIF from an input with `-display_rotation` (even with `-noautorotate`, the output was the same size as a file without `irot`).
Therefore we created the files by adding `irot` by hand to an AVIF without `irot`. The original AVIF was created as follows.

```sh
ffmpeg -f lavfi -i testsrc=s=640x360 -frames:v 1 base.png
ffmpeg -i base.png -c:v libsvtav1 -frames:v 1 plain.avif
```

The following Python (`build.py`) adds `irot` to the AVIF's `ipco`, adds an association to `ipma`, and fixes the length of `meta` and the position in `iloc`. We tried `angle` values of 1, 3, and 2.

```python
import struct
import sys

def box_end(data, offset):
    return offset + struct.unpack('>I', data[offset:offset + 4])[0]

def build(source, angle, name):
    d = bytearray(open(source, 'rb').read())
    find = lambda t: d.index(t) - 4
    meta_o, iprp_o, ipco_o, ipma_o, iloc_o = (find(t) for t in (b'meta', b'iprp', b'ipco', b'ipma', b'iloc'))
    ipco_end = box_end(d, ipco_o)
    # Count the children of ipco and use the count as the number of the new property (1-based).
    count, offset = 0, ipco_o + 8
    while offset < ipco_end:
        count += 1
        offset = box_end(d, offset)
    assert d[ipma_o + 8:ipma_o + 12] == b'\x00\x00\x00\x00' and d[ipma_o + 12:ipma_o + 16] == b'\x00\x00\x00\x01', 'assumes ipma is version 0 with exactly one item'
    ipma_size = struct.unpack('>I', d[ipma_o:ipma_o + 4])[0]
    d[ipma_o + 18] += 1                                            # number of associations
    d[ipma_o + ipma_size:ipma_o + ipma_size] = bytes([count + 1])  # association to the new property
    struct.pack_into('>I', d, ipma_o, ipma_size + 1)
    d[ipco_end:ipco_end] = struct.pack('>I4sB', 9, b'irot', angle)
    for box, add in ((ipco_o, 9), (iprp_o, 10), (meta_o, 10)):
        struct.pack_into('>I', d, box, struct.unpack('>I', d[box:box + 4])[0] + add)
    off_pos = iloc_o + 22                                          # extent_offset of iloc version 0 with one item
    old = struct.unpack('>I', d[off_pos:off_pos + 4])[0]
    assert old == box_end(d, meta_o) - 10 + 8, 'the assumed position of mdat is wrong'
    struct.pack_into('>I', d, off_pos, old + 10)                   # the position of mdat moves 10 bytes later
    open(name, 'wb').write(d)

source = sys.argv[1]
for angle, name in ((1, 'irot90'), (3, 'irot270'), (2, 'irot180')):
    build(source, angle, f"{source.removesuffix('.avif')}-{name}.avif")
```

```sh
python3 build.py plain.avif   # creates plain-irot90.avif, plain-irot270.avif, and plain-irot180.avif
```

This procedure depends on `meta` having `hdlr`, `pitm`, `iloc`, `iinf`, and `iprp` in that order, `mdat` being after `meta`, `iloc` being version 0 with exactly one item, and `ipma` being version 0 with exactly one item. For the 640×360 and 64×32 AVIFs created with libsvtav1, the `assert` passed.
We examined the created files as follows.

```sh
ffprobe -v error -show_streams -of json -i plain-irot90.avif
ffmpeg -i plain-irot90.avif decoded.png && ffprobe -v error -show_entries stream=width,height -of csv=p=0 decoded.png
```

The result was that `ffprobe` returned `width` 640, `height` 360, and `rotation` 90, and the decoded PNG was 360×640. `plain-irot270.avif` had a `rotation` of -90 and 360×640, and `plain-irot180.avif` had -180 and 640×360. For the 64×32 AVIF as well, `irot` value 1 gave 32×64 and value 2 gave 64×32.

## Implication

This is the premise of the decision in [Video Playback Design §7.1](../design/video.md#return-the-rotation-of-images-read-with-ffmpeg-as-an-attribute-without-swapping-the-dimensions).
The server decides the media info `rotation` depending on the `rotation` in the `side_data_list` of the video stream, in the `-show_streams` JSON that `probe_image_size` in [ffmpeg.rs](../../daemon/modules/engine/src/core/media/ffmpeg.rs#L58) receives.
An image without `irot` has no `side_data_list`, so it can be treated as having no rotation.
