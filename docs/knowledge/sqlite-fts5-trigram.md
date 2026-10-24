# SQLite's FTS5 Trigram Tokenizer Matches Only Substrings of 3 or More Characters Through the Index

**Source: Literature only** (SQLite FTS5 documentation, sqlite.org/fts5.html / 2026-10-23)

One item of [knowledge.md](../knowledge.md).

## Fact

The FTS5 trigram tokenizer indexes every 3-character sequence of the text.
In a full-text query (`MATCH`), a substring of fewer than 3 Unicode characters matches no rows. FTS5 does not fall back to a scan.
A trigram table also supports `LIKE` and `GLOB` through the index unless `remove_diacritics` is set. A table with `case_sensitive 1` can use the index only for `GLOB`, not for `LIKE`.
If a `LIKE` or `GLOB` pattern has no run of non-wildcard characters, FTS5 scans the whole table.
`remove_diacritics 1` can be set only when `case_sensitive` is 0.

The `libsqlite3-sys` 0.30.1 that `sqlx-sqlite` 0.9.0 resolves to in `daemon/Cargo.lock` builds the bundled SQLite 3.46.0 with `-DSQLITE_ENABLE_FTS5` (its `build.rs` and `sqlite3/sqlite3.h`).

## Basis

On 2026-10-23 we consulted the descriptions of the trigram tokenizer and its options in sqlite.org's FTS5 documentation.

- "Substrings consisting of fewer than 3 unicode characters do not match any rows when used with a full-text query."
- `remove_diacritics`: "It may only be set to 1 if the case_sensitive options is set to 0 - setting both options to 1 is an error."

The build flag and version were read from the crate source in the local Cargo registry.

## Implication

Name search in the name index matches terms of 3 or more characters through the trigram index, and matches shorter terms by comparing names directly ([Listing and Search Design §5.1](../design/listing.md#51-name-index)).
Names are normalized before being stored, so the table uses `case_sensitive 1` and does not use FTS5's own case folding or diacritic removal.
