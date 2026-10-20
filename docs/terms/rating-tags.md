# rating-tags Terms

Part of [terms.md](../terms.md).
This document holds the per-layer names of terms confined to rating-tags.
The term list and the boundaries with neighboring terms are in the parent.

<a id="t-rating"></a>
## Rating

**Names by layer**
The engine uses `Rating` (`Good`, `Bad`) and represents the unrated state as `None` of an `Option`.
The API uses `RatingValue` (`good`, `bad`, `none`), where `none` represents the unrated state.
The web client uses the API type as is, under the name `Rating`.
In storage, an unrated Entry has no row.
