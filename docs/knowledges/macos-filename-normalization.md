# HFS Plus Stores Filenames in Decomposed Form, and APFS Preserves the Normalization Used at Creation

**Source: Literature only** (Apple's Technical Note TN1150 and Apple File System Guide / 2026-10-03)

One item of [knowledges.md](../knowledges.md).

## Fact

HFS Plus stores a filename's Unicode string fully decomposed and in canonical order.
APFS stores a filename in the normalization it had at creation, and uses a hash of the normalized form only at comparison time.
As a result, a filename created on macOS remains in decomposed form (close to NFD) if it was created on HFS Plus or by an application that passes names in decomposed form.

## Basis

On 2026-10-03 we consulted the following primary sources.

- TN1150 HFS Plus Volume Format: "HFS Plus stores strings fully decomposed and in canonical order."
- The FAQ of the Apple File System Guide: "APFS preserves the normalization of the filename and uses hashes of the normalized form of the filename to provide normalization insensitivity, whereas HFS+ stores the normalized form of the filename on disk to provide normalization insensitivity."

## Implication

A Library can contain both decomposed and composed names, so name search normalizes before matching ([Listing and Search Design §5](../design/listing.md#5-name-search)).
