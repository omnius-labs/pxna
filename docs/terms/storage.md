# storage Terms

Part of [terms.md](../terms.md).
This document holds the per-layer names of terms confined to storage.
The term list and the boundaries with neighboring terms are in the parent.

<a id="t-archive"></a>
## Archive

**Names by layer**
The engine uses `FileStorageArchive`, and the API uses `archive`.

<a id="t-file-storage-driver"></a>
## FileStorageDriver

**Names by layer**
The engine uses `FileStorageDriver`, and the implementation types use `FileStorageLocal` and `FileStorageArchive`.
`FileStorageLocal` is exposed from the service layer under the name `LocalStorageDriver`, and the daemon uses it by that name.
It is not exposed in the API.

<a id="t-nested-path"></a>
## NestedPath

**Names by layer**
The engine uses `NestedPath` and `NestedPathBuf`.
The API represents paths inside archives as a concatenation of Library-relative paths (`RelativePath`), and the daemon layer resolves the boundaries and turns them into a `NestedPath`.
