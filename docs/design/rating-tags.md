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
- Following external changes to Entries (moves, renames, and deletions that do not go through Pxna) is not included in the scope of this document (§8.2).
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

Ratings and tags are saved in the following tables of `state_dir/sqlite.db`.
`user_id` is the identifier of the session's user, and rows are split per user.
Different sessions of the same user use the same `user_id`, so they read and write the same values.

| Table | Primary key | Contents |
| --- | --- | --- |
| `entry_ratings` | `user_id`, `library_id`, `path` | good or bad and the update time. An unrated Entry has no row |
| `tags` | `id`. `user_id` and `name` are unique | The per-user vocabulary |
| `entry_tags` | `user_id`, `library_id`, `path`, `tag_id` | The words attached to an Entry |

`path` is the resolved Entry's Library-relative path (starting with `/`).
The request's path is not used as is, and after going through the resolution of the Entry it becomes the key for saving and searching.
This is so that different spellings containing `./` or `..` do not create separate rows for the same Entry.

Replacing an Entry's tags is done in one transaction.
Even if it fails midway, both the tags and the vocabulary return to the state before the replacement.

If `sqlite.db` is deleted, ratings and tags are also lost, and there is no means of restoring them.
The tables are migrated by the component that uses them, at construction time.
Even if you go back to an older daemon, these tables remain unused and are not deleted.

### 6.2 Following Changes to Entries

Ratings and tags follow changes to Entries made through Pxna.
At the end of a mutating operation, rows are reassigned or deleted as follows.

| Operation | Ratings and tags |
| --- | --- |
| Move and rename within the same Library | Reassigned to the destination path. For folders and archives, the rows of Entries underneath are also reassigned |
| Move across Libraries | Reassigned to the destination Library and path |
| Copy | Not given. A copy is a different Entry and does not inherit the original's rating and tags |
| Delete | The rows of Entries that no longer exist, and of Entries underneath, are deleted. The vocabulary remains |
| Overwrite at the same path | Remain as they are. The Entry's path does not change |

Reassignment targets the rows of all users.
An Entry's path is one regardless of the user, so this is so that one user's move does not leave other users' rows behind.
When a row already exists at the destination, it is treated as overwritten by the moved Entry and is deleted first and then reassigned.
A name that merely matches by prefix (`/ab` when moving `/a`) is matched including the path delimiter (`/a/`) and is not moved.

Reassignment and deletion are done after the file operation completes, and even if they fail they are only logged and the operation is not made a failure.
In a failed reassignment, the transaction is rolled back and the rows of the source and the destination remain as they were.
The source path no longer exists, so when the following reflection of the change succeeds in deleting, the moved Entry's rating and tags are removed.
The rating and tags of the overwritten Entry that was at the destination remain by the rollback, and are shown on the Entry now at the destination, that is, the moved Entry.
When the following reflection of the change could not confirm the source's non-existence, or when the deletion fails, the moved Entry's rating and tags remain at the source path.
In a failed deletion, the deleted Entry's rows remain, and when a file is created at the same path, that rating and tags are shown on the new Entry.
Changes not made through Pxna also cannot be followed because the daemon does not know about them (§8.2).

## 7. API

Ratings are read and written by specifying the Entry's path, and tags are split into per-Entry reading and writing, and the list of the vocabulary.
The path, parameters, schemas, and status codes of each operation are authoritative in [openapi.yaml](../../daemon/openapi.yaml), and this document writes only the conventions common to the operations.

- For both reading and writing, the path pointing to an Entry is a Library-relative path and goes through the check of the target (§3).
- Writing is a replacement, and on success returns the saved value. The client reflects the returned value on the screen.
- Authentication uses the same session as other APIs, and ratings and tags read and write only the session user's values.
- The vocabulary has no Library. A user's words are shared across Libraries.

### 7.1 Filtering Listings

The folder listing API can narrow Entries by conditions on ratings and tags.
The rating can be one of good, bad, or none (unrated), and the tag one word; when specified together, it narrows to Entries that satisfy both.
It can also be combined with name terms, recursion, and sorting.

- Filtering selects only images and videos. Folders and files that are neither images nor videos cannot be rated or tagged, so they are not included under any condition. Unrated means images and videos that have not been rated, and does not include folders and the like.
- Filtering is not put into the listing's snapshot, and is done when cutting the page, with the set of paths looked up from the user's ratings and tags. Giving a rating or tag does not rescan, and the snapshot can be shared independently of the filtering conditions, as with names and sort order.
- During filtering, the version of ratings and tags is added to the listing's identifier. The version advances inside the daemon each time ratings or tags are written. If a rating were given in the middle of reading pages in succession, the preceding and following pages would be cut from different sets, so the identifier is changed to make the client reread. The identifier of an unfiltered listing does not change.
- Only the detail listing accepts the conditions. The summary listing does not, and the client uses the detail listing when there are conditions.
- The timeline does not accept conditions (§8.2).

## 8. Design Decisions

### 8.1 Decided

#### Make Ratings Three States: Two Choices of Good and Bad Plus Unrated

**Decision**
The rating's value is the two values good and bad, and is held in three states by adding the state of not being rated (§4).

**Reason**
Sorting is the work of looking and deciding whether to keep or drop.
With two choices, rate mode's operations fit in the three of "good", "bad", and "do not rate", and can proceed with one key operation per image.

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

#### The Storage Key Is the Library and the Library-Relative Path

**Decision**
An Entry is pointed at and saved by `(library_id, path)` (§6).

**Reason**
Entries have no stable identifier, and the daemon does not put marks on Entry contents.
With a path, Entries inside archives can be pointed at in the same form.

**Rejected alternative**
A method that points at them by a content hash.
Ratings would follow even if an Entry is moved, but large files would have to be read entirely, and the cost for Entries inside archives would also increase, so we rejected it.

#### Keep Words in the Vocabulary Even After They Are Removed from Entries

**Decision**
Words attached to no Entry are also kept in the vocabulary (§5).

**Reason**
If a word vanished from the completion candidates every time it is removed, one would have to retype the word to attach it again.
The vocabulary is small, so keeping them costs nothing.

**Rejected alternative**
A method that automatically deletes words attached to no Entry.
The vocabulary would be tidy, but words one tried attaching and detaching would disappear from completion, so we rejected it.

#### Make Ratings and Tags Follow Changes Made Through Pxna

**Decision**
When Pxna's mutating operations finish, ratings and tags are reassigned to the destination on moves and renames and removed on deletions. They are not inherited on copies (§6.2).

**Reason**
Ratings and tags are pointed at by path, so unless they follow, the results of sorting are lost at the point the user fixes a name or location.
Pxna's mutating operations go through the inside of the daemon, so the path is known at the end and this can be done at the same time as the reassignment the timeline index does.
A copy creates a different Entry, and if the original's rating were inherited, then when only the original is later rated again, the copy's rating would disagree.

**Rejected alternative**
A method that does not follow, and has the user reattach them after a move or rename.
No implementation would be needed, but the results of sorting would be lost every time a name is fixed, so we rejected it.

#### Do Filtering in Search Mode and Apply It When Cutting the Page

**Decision**
Filtering by ratings and tags is done as a condition of search mode. The server does not change the snapshot, and at page cutting, matches against the set looked up from the user's ratings and tags (§7.1).
The version of ratings and tags is added to the identifier of a listing during filtering.

**Reason**
Search mode is a screen for choosing a start and a scope and searching, so "only good in this folder" and "tagged videos across the whole Library" can be expressed with the same operation.
Ratings and tags change each time they are given, so if they were put in the snapshot's conditions, each time a rating is given it would rescan.
If matched at page cutting, the scan result can be shared as with name conditions.
Ratings and tags are a set of paths that can be looked up one by one per Entry, so at the scale of personal use, reading the set on every request is cheap.

**Rejected alternative 1**
A method that also places filtering in the folder view.
Search entrances would be two, and the way of choosing the start and scope would not match, so we rejected it.

**Rejected alternative 2**
A method that has each Entry of the listing snapshot hold its rating and tags.
Filtering matching would be faster, but the snapshot would have to be rebuilt each time a rating is given, so we rejected it.

### 8.2 Deferred

#### Following External Changes to Entries

**Current state**
Changes made through Pxna are followed (§6.2). Moves, renames, and deletions not made through Pxna are not known to the daemon, so ratings and tags remain at the original path and are not attached to the new path.
Rows of deleted Entries also remain.

**Why not decide now**
To know about external changes, periodic scans of the Library or file system notifications are needed.
Timeline aggregation ([Timeline Design §6](./timeline.md#6-aggregation)) does scan, but to reassign the paths of ratings and tags there is no clue for telling that contents are identical, and a deletion cannot be told apart from the addition of a new Entry.

**Conditions for deciding**
When the user moves or renames an Entry without going through Pxna and is actually troubled that ratings and tags were lost.

#### Filtering in the Timeline, and Sorting by Ratings and Tags

**Current state**
Filtering is only in folder listings (search mode), and the timeline API does not accept conditions.
In neither listing is sorting in the order of ratings or tags possible. Only one tag can be used as a filtering condition.

**Why not decide now**
The timeline index is in a separate SQLite and pages are cut by the index's cursor, so adding conditions would change the count per page and the meaning of the cursor.
Sorting would be a design that adds ratings and tags to the listing's sort order, and would affect the sharing of the snapshot's order.

**Conditions for deciding**
When the need to view the timeline narrowed by ratings or tags actually arises, or when the need to search by combinations of multiple tags actually arises.

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
The API is the 5 operations of reading and writing ratings, reading and writing an Entry's tags, and listing the vocabulary, plus the filtering conditions on the folder listing, and the contract and generated artifacts are in [openapi.yaml](../../daemon/openapi.yaml).
At the end of mutating operations, `LibraryMutation` calls the reassignment and deletion of ratings and tags.

The web client has ratings of images and videos (display only) and attaching and detaching tags in the info panel, rate mode in the viewer, and filtering by ratings and tags in search mode ([Client Design §6.1](./client.md#61-current-code)).

### 9.2 Roadmap

There is no remaining work in this milestone.
The deferred issues are covered by §8.2.

For confirmed defects, see [issues.md](../issues.md).
