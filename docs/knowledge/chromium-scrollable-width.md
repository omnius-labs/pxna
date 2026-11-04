# Chromium Limits an Element's Width to 33,554,428 CSS Pixels

**Source: Verified** (Chromium 156.0.0.0 / 2026-11-04)

One item of [knowledge.md](../knowledge.md).

## Fact

In headless Chromium 156.0.0.0 on Ubuntu, an element given a `width` above 33,554,428 CSS px is laid out at 33,554,428 CSS px, so the `scrollWidth` of its scroll container stops there, and the largest `scrollLeft` of a 500 px wide container is 33,553,928. Widths up to 33,554,428 are kept as they are. The browser reports no error and no warning.
Content placed beyond that width cannot be scrolled to. We did not check other browsers and versions, and this is not the limit of other properties such as `height`.

## Basis

We opened `about:blank` with playwright-cli, put an empty `div` (height 10 px, `position: relative`) in a `div` with `overflow: auto`, `width: 500px`, and `height: 50px`, and for each width set `scrollLeft` to 1e10 and read back `scrollWidth`, the largest `scrollLeft`, and the inner element's `getBoundingClientRect().width`.

| Requested width | `scrollWidth` | Largest `scrollLeft` | Inner element's width |
| --- | --- | --- | --- |
| 10,000,000 | 10,000,000 | 9,999,500 | 10,000,000 |
| 30,000,000 | 30,000,000 | 29,999,500 | 30,000,000 |
| 33,554,428 | 33,554,428 | 33,553,928 | 33,554,428 |
| 33,554,429 | 33,554,428 | 33,553,928 | 33,554,428 |
| 40,000,000 | 33,554,428 | 33,553,928 | 33,554,428 |
| 2,000,000,000 | 33,554,428 | 33,553,928 | 33,554,428 |

## Implication

This fact is why the viewer's gallery lays out only a window of positions around the viewed place instead of every position of the listing: 33,554,428 CSS px is about 54,000 items of 600 CSS px with their gaps, which a large folder passes ([Client Design §5.1](../design/client.md#lay-out-gallery-mode-from-loaded-thumbnails)).
