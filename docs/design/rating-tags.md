# Rating and Tag Design

## 1. About This Document

This document covers the ratings and tags that a user gives to Entries, and defines what can be rated and tagged, the value rules, storage, the shape of the API, and the reasons for adopting them.
The intended readers are developers and AI agents who implement or review this concern.
It assumes the vocabulary of [terms.md](../terms.md).
The definitions of words are not in this document.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | The targets of ratings and tags, value rules, storage, API conventions, reasons for adoption, deferred items |
| [design.md](../design.md) | Pxna's overall scope, where state lives, authentication and sessions, conventions for mutating APIs |
| [Client Design](./client.md) | Key operations of rate mode, and tag operations in the info panel |
| [Thumbnail Design](./thumbnail.md) | The table of extensions used to determine images and videos |
| [terms.md](../terms.md) | Definitions of terms |
| [issues.md](../issues.md) | Clear defects confirmed in code and handling policies |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | The authoritative rating and tag API paths, parameters, schemas, and status codes |

The entry point of the design document family is [design.md](../design.md).

### 1.2 Tense of This Document

§1 through §7 describe the completed form in the present tense.
The gap between the current code and the completed form, and the remaining work in dependency order, are gathered in §9.
If you want to know what works now, read §9 first.

## 2. Responsibilities and Boundaries

Ratings and tags are the user's own data, for sorting out and keeping the images and videos they have seen.
Unlike attributes of the Entry itself such as the timeline date or size, they cannot be obtained by reading the Entry, and do not exist until the user gives them.
For that reason, ratings and tags are written neither to files nor to Libraries, and the daemon holds them in `sqlite.db` (§6).

This document covers determining the targets that can be rated and tagged, value rules, storage, and the conventions of the read and write API.
The boundaries with other concerns are as follows.

| Matter | Authoritative source |
| --- | --- |
| The table of extensions used to determine images and videos | [Thumbnail Design §3](./thumbnail.md#3-determining-the-target) |
| Resolving Library-relative paths, and paths inside archives | [Storage Layer Design](./storage.md) |
| Authentication of sessions and users | [design.md §7](../design.md#7-authentication-and-sessions) |
| Where state lives, and the impact when it is lost | [design.md §9.2](../design.md#92-where-state-lives) |
| Key operations of rate mode, and tag operations in the info panel | [Client Design §3.4](./client.md#34-viewer), [§3.8](./client.md#38-info-panel) |

### 2.1 What Is Not Built

- Ratings do not have 3 or more levels.
  The sorting operation can be narrowed to the three of two choices and "do not rate".
- Tags do not have hierarchy or attributes.
  They are treated as a set of words, so creating words and attaching and detaching them is enough.
- Sorting by ratings and tags, and filtering in the timeline, are not included in the scope of this document (§8.2).
- Following a change to an Entry's content is not included.
  A file whose bytes change has a different content hash and starts unrated and untagged.
- Cleaning content hashes that no Entry refers to is not included (§8.2).
- It does not involve changing Entries.
  Neither ratings nor tags change the Entry's own contents, name, or modification time.
- Ratings and tags are not shared among multiple users.
  Each user's values need not appear on other users' screens.

## 3. Targets

What can be rated and tagged is an existing file that can be determined to be an image or a video by the same table of extensions as thumbnails.
They cannot be given to folders, to the archive file itself, or to files that are neither images nor videos.
Images and videos inside archives are targets.

They can also be given in `readonly` Libraries and to paths inside archives.
Ratings and tags do not change Entries and are written to the daemon's state, so they are not subject to the rule that rejects mutating APIs ([design.md §8](../design.md#8-conventions-for-mutating-apis)).

The check of whether something is a target is done for both read and write requests.
A nonexistent path is `NotFound`, and an Entry that is not a target is `InvalidInput`.
If saved without checking existence, the ratings of mistyped paths would remain and rows invisible from listings would accumulate.

## 4. Ratings

A rating has 3 states: **good**, **bad**, and unrated.
Unrated is saved not as a value but as the state of having no rating row.
The API represents unrated as `none`, and to remove a rating, `none` is written.

The rating given to one Entry is one per user.
Writing is always a replacement, and reading returns the saved value as is.

## 5. Tags

The tags given to an Entry are a set of words, and order has no meaning. The API returns them sorted by word.

A word is 1 to 64 characters after trimming leading and trailing whitespace, and contains no control characters.
Case is distinguished.
At most 32 words can be given to one Entry.
The length of a word is counted in Unicode scalar values.
These limits are so that words that do not fit on the screen and huge sets are not saved, and are also written in the API's schema.

Writing an Entry's tags is a replacement with the given set.
Words are normalized and then de-duplicated, and a word that has never been used is created at that time.
When a word is invalid or there are too many, it is `InvalidInput`, and neither the saved tags nor the vocabulary change.

The words the user has created so far are held as the vocabulary.
The vocabulary keeps words even after they are removed from Entries, and serves as completion candidates while typing.

## 6. Storage

### 6.1 What Is Stored

The decision to key ratings and tags by content, and its reasons, are in [design.md §11.1](../design.md#identify-ratings-and-tags-by-content-not-by-path).
Ratings and tags are saved in the following tables of `state_dir/sqlite.db`.
`user_id` is the identifier of the session's user, and rows are split per user.
Different sessions of the same user use the same `user_id`, so they read and write the same values.

| Table | Primary key | Contents |
| --- | --- | --- |
| `entry_content` | `library_id`, `path` | The content hash of the Entry at the path, with the size, modification time, and inode used to decide whether the cached hash is still valid. A path with no row, or with a changed fingerprint, is hashed again when read or written |
| `entry_ratings` | `user_id`, `content_hash` | good or bad and the update time. An unrated Entry has no row |
| `tags` | `id`. `user_id` and `name` are unique | The per-user vocabulary |
| `entry_tags` | `user_id`, `content_hash`, `tag_id` | The words attached to an Entry |

`content_hash` is a full SHA3-256 of the Entry's bytes.
For an entry inside an archive, it is the hash of the extracted entry, and the path cache uses the archive identity and inner path instead of the inode.
The hash is computed lazily, when ratings or tags are first read or written for a path, or when the cached fingerprint no longer matches.
The `entry_content` row is a cache and may be written on a read as well as on a write; when a rating or tag is written, the cache update and the value write are in one transaction.
A path that has never been rated or tagged may have no `entry_content` row; the listing filter treats it as unrated and untagged until the Entry is read or written.

The API still addresses an Entry by `(library_id, path)`.
The daemon resolves the path to the content hash before reading or writing ratings and tags, so different spellings containing `./` or `..` do not create separate rows.

Replacing an Entry's tags is done in one transaction.
Even if it fails midway, both the tags and the vocabulary return to the state before the replacement.

`entry_content` is a cache; if it is lost, the hashes are computed again and the ratings and tags remain.
If `sqlite.db` is deleted, ratings and tags are also lost, and there is no means of restoring them.
The tables are migrated by the component that uses them, at construction time.
Even if you go back to an older daemon, these tables remain unused and are not deleted.

### 6.2 Changes to Entries

Ratings and tags are attached to the content, so a change to the path does not reassign them.
The following table is the behavior after each change.

| Change | Ratings and tags |
| --- | --- |
| Move or rename, through Pxna or outside it | Remain on the same content hash. The path cache is refreshed on the next read |
| Copy | Shared with the copy, because the content hash is the same |
| Delete, including a move to the trash | Remain on the content hash. They apply again if the content is restored or exists in another Entry |
| Empty the trash | Remain until a cleanup removes content hashes that no Entry refers to (§8.2) |
| Overwrite at the same path | The new content has a new hash and starts unrated and untagged. The old content's rows remain for any other copy |
| Content changes at the same path | The fingerprint changes and the Entry starts unrated and untagged. A change that preserves the size and modification time is not detected |

No reassignment pass runs after a mutating operation.
The old design reassigned path-keyed rows after each move and deleted them after each delete; content identity removes that pass and the failure path that came with it.
A copy inherits the original's ratings and tags, because the same bytes are the same content.

## 7. API

Ratings are read and written by specifying the Entry's path, and tags are split into per-Entry reading and writing, and the list of the vocabulary.
The path, parameters, schemas, and status codes of each operation are authoritative in [openapi.yaml](../../daemon/openapi.yaml), and this document writes only the conventions common to the operations.

- For both reading and writing, the path pointing to an Entry is a Library-relative path and goes through the check of the target (§3).
- Writing is a replacement, and on success returns the saved value. The client reflects the returned value on the screen.
- Authentication uses the same session as other APIs, and ratings and tags read and write only the session user's values.
- The vocabulary has no Library. A user's words are shared across Libraries.

### 7.1 Filtering Listings

The search API ([Listing and Search Design §5.3](./listing.md#53-searching-across-libraries)) can narrow Entries by conditions on ratings and tags.
The rating can be one of good, bad, or none (unrated), and tags can be any number of words; an Entry matches only when it has all the given tags and, if a rating is given, that rating.
It can also be combined with name terms, recursion, the Libraries searched, and sorting.

- Filtering selects only images and videos. Folders and files that are neither images nor videos cannot be rated or tagged, so they are not included under any condition. Unrated means images and videos that have not been rated, and does not include folders and the like.
- Filtering is not put into the listing's snapshot, and is done when cutting the page, with the set of content hashes looked up from the user's ratings and tags. The page's Entries are joined to `entry_content` to get their hashes; an Entry whose path has no cached hash, or whose fingerprint no longer matches, is treated as unrated and untagged for that page. It is hashed when it is read or written, and a later page reflects the result. Giving a rating or tag does not rescan, and the snapshot can be shared independently of the filtering conditions, as with names and sort order.
- A recursive listing answered from the name index is the exception. It starts from the paths in `entry_content` whose hashes match the condition (for unrated, from the images and videos of the index minus the paths whose hash has a rating), keeps those the name index holds, and puts only the result in the snapshot, together with the conditions and the version of ratings and tags ([Listing and Search Design §5.2](./listing.md#52-answering-recursive-listings-with-conditions)). Writing a rating or tag rebuilds that snapshot from the index without walking the file system. A path that the index no longer holds does not appear there after the next aggregation.
- The sets of content hashes are looked up once per user, and each Library's listing is filtered with the paths in `entry_content` that carry them.
- During filtering, the version of ratings and tags is added to the listing's identifier. The version advances inside the daemon each time ratings or tags are written. If a rating were given in the middle of reading pages in succession, the preceding and following pages would be cut from different sets, so the identifier is changed to make the client reread. The identifier of an unfiltered listing does not change.
- Only the search API accepts the conditions. The folder listing and the summary listing do not.
- The timeline does not accept conditions (§8.2).

## 8. Design Decisions

### 8.1 Decided

#### Make Ratings Three States: Two Choices of Good and Bad Plus Unrated

**Decision**
The rating's value is the two values good and bad, and is held in three states by adding the state of not being rated (§4).

**Reason**
Sorting is the work of looking and deciding whether to keep or drop.
With two choices, rate mode's operations fit in the three of "good", "bad", and "unrated", and can proceed with one key operation per image.

**Rejected alternative**
Making ratings with many levels, like 5-level stars.
The granularity of later sorting would increase, but the effort of thinking about the level for each image would increase and keys would be hard to assign, so we rejected it.

#### Tags Are Free Input, and Words Are Created When Attached

**Decision**
Tags are words the user enters freely, and the words created so far are completed while typing.
Words are created when attached, and there is no advance registration or management screen (§5).

**Reason**
If words had to be registered first, the effort of registering would intervene when one wants to attach.
With completion, the multiplication of words from mistyping the same word can be suppressed.

**Rejected alternative**
A method that registers words in advance and lets only registered words be chosen.
The vocabulary would be tidy, but the registration screen and steps would increase, so we rejected it.

#### Save Ratings and Tags per User, and Allow Them in `readonly` Libraries

**Decision**
Ratings and tags are saved per `user_id`.
They can also be given in `readonly` Libraries and to Entries inside archives (§3, §6).

**Reason**
Libraries exposed for browsing only are exactly where there are many occasions to want to sort.
Ratings and tags are written to the daemon's state and do not change Entries, so they do not go against the purpose of `readonly`.
At present the only user is the single owner, but if separated per user, values do not mix with others' even if users are added.

**Rejected alternative**
A method that prevents giving them in `readonly` Libraries.
The `readonly` rule would be kept uniform, but one could not sort in browse-only Libraries, so we rejected it.

#### Do Not Reassign Ratings and Tags After Mutations

**Decision**
A move, rename, copy, or delete does not reassign ratings and tags (§6.2).
The content hash is the key, so a path change is reflected by refreshing the path cache on the next read.

**Reason**
The old design reassigned path-keyed rows after each mutation and deleted them after each delete.
That pass ran after the file operation, could fail on its own, and could not follow a move made outside Pxna.
Content identity removes the pass and the failure path, and it makes an external move behave the same as one made through Pxna.

**Rejected alternative**
Reassigning path-keyed rows after each mutation.
It is the old design, and it cannot follow external moves, so we rejected it.

#### Keep Words in the Vocabulary Even After They Are Removed from Entries

**Decision**
Words attached to no Entry are also kept in the vocabulary (§5).

**Reason**
If a word vanished from the completion candidates every time it is removed, one would have to retype the word to attach it again.
The vocabulary is small, so keeping them costs nothing.

**Rejected alternative**
A method that automatically deletes words attached to no Entry.
The vocabulary would be tidy, but words one tried attaching and detaching would disappear from completion, so we rejected it.

#### Do Filtering in Search Mode and Apply It When Cutting the Page

**Decision**
Filtering by ratings and tags is done as a condition of search mode. The server does not change the snapshot, and at page cutting, matches against the set looked up from the user's ratings and tags (§7.1).
The version of ratings and tags is added to the identifier of a listing during filtering.
A recursive listing answered from the name index starts from the set of rated or tagged paths, or for unrated from the index's images and videos minus the rated paths, and its snapshot holds only the result (§7.1).

**Reason**
Search mode is a screen for choosing a start and a scope and searching, so "only good in this folder" and "tagged videos across the whole Library" can be expressed with the same operation.
Ratings and tags change each time they are given, so if they were put in the snapshot's conditions, each time a rating is given it would rescan.
If matched at page cutting, the scan result can be shared as with name conditions.
Ratings and tags are a set of paths that can be looked up one by one per Entry, so at the scale of personal use, reading the set on every request is cheap.
Matching at page cutting needs a snapshot of all Entries under the start, which a recursive listing answered from the name index does not build.
Rated and tagged Entries are a small part of a Library, so starting from them and rebuilding when a rating is written costs one lookup in the index. Only the unrated condition starts from the index's images and videos, and is bounded by the count limit of listings.

**Rejected alternative 1**
A method that also places filtering in the folder view.
Search entrances would be two, and the way of choosing the start and scope would not match, so we rejected it.

**Rejected alternative 2**
A method that has each Entry of the listing snapshot hold its rating and tags.
Filtering matching would be faster, but the snapshot would have to be rebuilt each time a rating is given, so we rejected it.

#### Combine Several Tag Conditions as All of Them

**Decision**
A search can be given any number of tags, and an Entry matches only when it has all of them (§7.1).

**Reason**
Tags are a set of words on one Entry, and adding a word to the condition is the natural way to narrow, as adding a name term narrows by name.
With all of them required, tags combine with the rating and the name terms by one rule, so the screen needs no choice of how they combine.
The sets of tagged paths are already looked up per word, and intersecting them costs no more than looking them up.

**Rejected alternative 1**
A method that switches between all of the tags and any of them.
Both needs could be met, but the combinations with the rating and the name terms and their tests would double, while the need for any of them has not appeared, so we deferred it (§8.2).

**Rejected alternative 2**
A method that keeps one tag word as the condition.
No API change would be needed, but a photo tagged with both a place and a person could not be found by both, so we rejected it.

### 8.2 Deferred

#### Cleaning Content Hashes and Path Cache Rows with No Entry

**Current state**
Ratings and tags are keyed by content hash, and `entry_content` caches the hash per path (§6.1).
When every copy of a content is deleted or a path disappears, the rows remain in SQLite.

**Why not decide now**
A file can be temporarily absent, for example when an external disk is unmounted or the trash has not been emptied.
Deleting the rows then would lose the user's classification when the content returns.
The rows are small.

**Conditions for deciding**
When the rows have grown enough to matter, or when the user asks to forget a content.

#### Filtering in the Timeline, and Sorting by Ratings and Tags

**Current state**
Filtering is only in the search API (search mode), and the timeline API does not accept conditions.
In neither listing is sorting in the order of ratings or tags possible. Several tags combine only as all of them, not as any of them.

**Why not decide now**
The timeline index is in a separate SQLite and pages are cut by the index's cursor, so adding conditions would change the count per page and the meaning of the cursor.
Sorting would be a design that adds ratings and tags to the listing's sort order, and would affect the sharing of the snapshot's order.

**Conditions for deciding**
When the need to view the timeline narrowed by ratings or tags actually arises, or when the need to search for Entries that have any one of several tags actually arises.

#### Organizing the Vocabulary, and Completion for a Large Vocabulary

**Current state**
There are no operations to rename, merge, or delete words, and the vocabulary list returns all items at once.

**Why not decide now**
In personal use, the vocabulary is expected to stay within a few hundred words, so returning all of them is cheap.

**Conditions for deciding**
When words have grown to the point that organizing the vocabulary is needed, or when the vocabulary response becomes noticeable as a delay in completion.

## 9. Current State and Remaining Work

### 9.1 Current Code

The engine's `RatingTagService` handles saving and reading ratings and tags.
The tables are migrated at construction time.
The daemon's handlers check the target (an existing file that is an image or video) and then call `RatingTagService`.
The API is the 5 operations of reading and writing ratings, reading and writing an Entry's tags, and listing the vocabulary, plus the filtering conditions on the search API, and the contract and generated artifacts are in [openapi.yaml](../../daemon/openapi.yaml).
At the end of mutating operations, `LibraryMutation` calls the reassignment and deletion of ratings and tags.
The current code keys ratings and tags by `(library_id, path)` and reassigns them after mutations; the content hash of §6 is not implemented.

The web client has ratings of images and videos (display only) and attaching and detaching tags in the info panel, rate mode in the viewer, and filtering by a rating and several tags in search mode ([Client Design §6.1](./client.md#61-current-code)).

### 9.2 Roadmap

The content hash identity of §6.1 and §6.2 is M26 of [design.md §12.2](../design.md#122-roadmap).
The deferred issues are covered by §8.2.

For confirmed defects, see [issues.md](../issues.md).
