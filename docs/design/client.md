# Client Design

## 1. About This Document

This document covers how the web client shows a Library and what they request from the server and when, and defines the screen structure, how listings and thumbnails are requested, the viewer, Entry operations, the behavior of display preferences, and the reasons for adopting them.
The intended readers are developers and AI agents who implement or review the clients.
It assumes the vocabulary of [terms.md](../terms.md).
The definitions of words are not in this document.

Note that the web client's interface is shown in English or Japanese (§3.11). Where this document quotes the text of buttons, menus, and messages, it quotes the Japanese text, with an English gloss.

### 1.1 Division of Responsibility Between Documents

| Document | What it covers |
| --- | --- |
| This document | Screen structure, behavior of tiles and the viewer, how listings and thumbnails are requested, the search screen, the screens for Entry operations, display preferences, reasons for adoption, deferred items |
| [design.md](../design.md) | Pxna's overall scope, how the web client is served, carrying sessions, conventions for mutating APIs, constraints common to clients |
| [Listing and Search Design](./listing.md) | How the server sorts, filters, and cuts listings from snapshots |
| [Thumbnail Design](./thumbnail.md) | How the server generates and caches thumbnails, sprite sheets, and display images |
| [Image Channel Design](./image-channel.md) | The connection that carries thumbnails, sprite sheets, and display images, request classes, delivery, revalidation |
| [Video Playback Design](./video.md) | The server-side mechanisms for media info of images and videos, direct playback, and transcoded playback |
| [Timeline Design](./timeline.md) | What is listed in the timeline, timeline dates, the server's aggregation and reading |
| [terms.md](../terms.md) | Definitions of terms |
| [issues.md](../issues.md) | Clear defects confirmed in code and handling policies |
| [daemon/openapi.yaml](../../daemon/openapi.yaml) | The authoritative paths, schemas, and status codes of the APIs the client calls |
| [daemon/image-channel.schema.json](../../daemon/image-channel.schema.json) | The authoritative messages of the image channel |

The entry point of the design document family is [design.md](../design.md).

### 1.2 Tense of This Document

§1 through §5 describe the completed form in the present tense.
The gap between the current code and the completed form, and the remaining work in dependency order, are gathered in §6.
If you want to know what works now, read §6 first.

## 2. Responsibilities and Boundaries

The client's role is to let the user view contents as a lineup of thumbnails while walking the Library's hierarchy.
It lets the user see at a glance what is in a folder without opening files one by one.
When an image or video is opened, it is displayed or played in the viewer.
In the web client, the user can also search by name, and create folders, rename, delete, move, copy, upload, and download.

The web client mainly targets desktop browsers, and on narrow screens (phone browsers) it provides browsing, search, and the viewer, and does not show Entry operations.

The boundaries with other concerns are as follows.

| Matter | Authoritative source |
| --- | --- |
| Serving the web client from the daemon on the same origin | [design.md §10.1](../design.md#101-web) |
| How sessions are received and stored | [design.md §7.2](../design.md#72-sessions) |
| How locations are pointed at, which roots may be displayed, when to use `Range` and when uploads | [design.md §10.2](../design.md#102-constraints-common-to-clients) |
| Sync and async of mutating operations, the conflict policy, deletion, the state of Operations | [design.md §8](../design.md#8-conventions-for-mutating-apis) |
| Where display preferences are saved | [design.md §9.2](../design.md#92-where-state-lives) |
| Listing snapshots, name matching, recursion limits | [Listing and Search Design](./listing.md) |
| How representative thumbnails are chosen, size steps, output format, generation concurrency | [Thumbnail Design](./thumbnail.md) |
| Media info for images and videos | [Video Playback Design §3](./video.md#3-media-info) |
| Transcoded playback sessions and HLS delivery | [Video Playback Design §5](./video.md#5-transcoded-playback) |
| What is listed in the timeline, timeline dates, aggregation | [Timeline Design](./timeline.md) |

## 3. Screens

### 3.1 Web

The web client combines a file manager's skeleton with a photo app's way of showing things.
The skeleton is the tree, the breadcrumb, the toolbar, and the context menu, which handle walking the Library's hierarchy and management operations.
For the contents, thumbnails are the main attraction, viewed with large tiles, a viewer with a filmstrip, and an info panel that shows shooting info.
The color scheme follows the OS's light and dark setting.

On a wide screen, one screen is divided into the following regions.

```text
+--------------------------------------------------------------------+
| Toolbar (mode switch, breadcrumb, operations, settings menu)       |
+------------+----------------------------------------+--------------+
| Tree       | Grid or list                           | Info panel   |
|  Library   |  [tile] [tile] [tile] [tile]           |  thumbnail   |
|   folder   |  [tile] [tile] [tile] [tile]           |  name, size  |
|    archive |  [tile] [tile] [tile] [tile]           |  shooting info |
+------------+----------------------------------------+--------------+
| Status bar (count, selected count and total size)      [Activity]   |
+--------------------------------------------------------------------+
```

Because the tree and the listing are laid out on one screen, the listing of contents is not interrupted by screen transitions when moving through the hierarchy.
The width of the tree, the timeline's list of months, and the info panel can be changed by dragging the handle at the border with the listing. The handles can also be moved with the left and right arrow keys. The width ranges from 160 to 480 CSS px for the tree and the timeline's list of months (saved separately) and 240 to 640 CSS px for the info panel, and when changed it is saved per device in the browser's `localStorage` and restored the next time it is opened. The search conditions do not change width. Narrow screens show no handles.
This screen is called folder view, and the toolbar's "フォルダ / タイムライン / 検索" (Folder / Timeline / Search) switch, three choices laid out side by side, goes back and forth between it, the timeline (§3.10), and search mode (§3.6).
On a wide screen, the current mode is shown only by the switch's highlight, and the name of the mode is not written next to it.
The breadcrumb to the right of the switch starts below the Library, and nothing is shown at the Library root, because the tree already shows which Library is open.
The info panel can be collapsed, and while collapsed the listing extends to the right edge.
The activity panel is shown overlaid at the bottom right, and when collapsed becomes a button at the edge of the status bar (§3.7).

The direct children of the folder selected in the tree are laid out in the listing as tiles or list rows.
The tree represents the Library's hierarchy, and lays out folders and archives with the same treatment.
An archive can be entered like a folder, so it is not distinguished even on the tree.

Double-clicking a listing tile moves into a folder or archive, and opens an image or video in the viewer. A single click selects.
While the mouse is over a video tile, flipbook playback is performed (§3.3).

At the top level of the tree, the Libraries that `GET /libraries` returns are listed.
The current position is represented by a URL of `/browse/{libraryId}/{Library-relative path}`, and the Entry open in the viewer is put in the `view` query as a path relative to the current folder. For a direct child Entry, it is its name.
Search is done at the dedicated `/search` URL, and the Libraries, the start, the scope, the conditions, and the Entry open in the viewer are put in the query (§3.6). Search results span Libraries, so `view` carries the Library id followed by the Library-relative path, as in the timeline.
Library-relative paths are percent-encoded per segment, and positions inside archives are also put in with the paths joined as they are ([design.md §11.1](../design.md#represent-paths-inside-archives-by-joining-paths)).
At `/`, the client opens the folder last opened in folder view, by redirecting with `replace` to that folder's `/browse/...` URL. The last folder is saved per device in the browser's `localStorage` under `pxna.lastFolder` as the Library id and the Library-relative path, on every folder view, without the viewer's `view` or `reveal`. When nothing is saved, or the saved Library no longer exists, it opens the root of the first Library that `GET /libraries` returns. When `GET /libraries/{libraryId}/folders` answers `404` for the saved path other than `/`, it opens that Library's root; any other failure keeps the saved path and leaves the retry to folder view. When there is no Library, or the list cannot be read, `/` shows only the Libraries in the tree and nothing in the listing.

On a narrow screen, the tree is folded into a drawer, and the info panel and the activity panel are not shown.
Tapping a tile or list row once moves into a folder or archive, and opens an image or video in the viewer. A tap with Ctrl, ⌘, or Shift selects as on a wide screen, and a double tap does not open again.
The context menu, toolbar operations that change Entries, drag and drop, and uploads are not shown.
Tile size, the thumbnail shape, and sorting operations are shown even on narrow screens.
Search is done by moving to search mode with the header's switch (§3.6).

When opened without a session, the initial setup screen is shown if the owner is not registered, or the login screen if registered, and once done it returns to the URL that was about to be opened.

### 3.2 Tiles

The look of a tile is decided by the kind of Entry.

| Entry | What the tile shows |
| --- | --- |
| Image | The thumbnail |
| Video | The thumbnail of the first frame and a mark showing it is a video |
| Folders and archives | The representative thumbnail. A folder icon if the server returns `NotFound` |
| Files for which a thumbnail cannot be made | An icon showing the kind |

Images and videos are told apart by the `image/` and `video/` prefixes of the listing's `metadata.contentType` ([Thumbnail Design §3](./thumbnail.md#3-determining-the-target)).
The name is shown below the tile.

Tiles are square, and the thumbnail shape is chosen in the display preferences between crop and whole display.
In crop, the thumbnail is cut out at the center so that it covers the tile (`resize=crop`).
In whole display, it fits in the tile preserving the aspect ratio, and the leftover part stays as the background (`resize=fit`).

The size of tiles is chosen from 5 levels by the display preferences' tile size.
On a wide screen, the minimum length of a tile's side is 96, 128, 176, 240, and 320 CSS px for each level, the number of columns that fit in the listing's width is found with that minimum, and then the columns are widened to fill the listing's width.
On a narrow screen, the levels are treated as numbers of columns (6, 5, 4, 3, 2 columns), and the tile's side is decided from the screen width.
Phone screen widths differ by model, so with a fixed size the number of columns and the right-edge margin would vary by model.
The thumbnails of a tile are requested at the smallest size step that covers the tile, and one generation also warms the smaller steps in both shapes ([Thumbnail Design §3](./thumbnail.md#3-determining-the-target)), so switching the shape or opening the viewer finds them in the cache.
The tile size is switched with the slider in the settings menu (§3.9), and on a wide screen also with Ctrl or ⌘ plus the wheel, and with the `+` and `-` keys.

The listing can be switched between grid and list.
A list is a sequence of rows with columns for a small thumbnail, name, modification date, size, and kind, and can be sorted by the column headings.
Pressing a heading sorts ascending by that key, and pressing the current key reverses the direction.
A narrow screen's list uses rows with a small thumbnail and two lines of text. The name goes on the upper line and the modification date and size on the lower, and column headings are not shown. Sorting is chosen in the settings menu.
While a search covers descendants or several Libraries, a "場所" (Location) column is added to a wide screen's list, and on a narrow screen's list the location is shown at the start of the second line (§3.6). The "場所" column does not sort.
A list has no tiles, so the tile size operations (the menu slider, wheel, and keys) are not shown. The thumbnail shape is applied to the rows' small thumbnails.
When switching between grid and list, as when moving between folders, the selection is cleared and the display returns to starting from the top of the listing.
The default sort order is the natural order of names, with folder-like Entries placed first.
Sorting can be chosen in the settings menu, and the grid and list use the same order.
Separating and sorting are left to the server (`foldersFirst`, `sort`, `order`).
The web client's tree has the server narrow to only folder-like Entries (`foldersOnly`) and lists them.

If fetching a listing fails with the server's deadline expiry (`Timeout`), the client indicates that and shows an operation to reload, and does not reload automatically.
This is because an extraction that passed its deadline can give the same result when repeated ([design.md §11.1](../design.md#return-extraction-and-generation-deadline-expiry-as-504)).

### 3.3 Flipbook Playback

When starting flipbook playback, the client wants that video's sprite sheet.
It passes the same values as the tile's thumbnail for width, height, and shape (`resize`). This is because the server puts both into the cache with one generation only when the thumbnail and sprite sheet are requested under the same conditions ([Thumbnail Design §6.2](./thumbnail.md#62-generate-together)).
It divides the sprite sheet equally by the number of frames in the response, and switches the display position at a fixed interval.
When the thumbnail shape is whole display, each frame fits in a square preserving the aspect ratio.
Therefore, it finds the frames' aspect ratio from the dimensions of the sprite sheet it obtained, and shows one frame at a time in an element sized to fit within the tile's side.
When stopped, the display returns to the first frame.

The server also makes the sprite sheet at the time it generates the tile's thumbnail ([Thumbnail Design §6.2](./thumbnail.md#62-generate-together)).
Therefore, this request is returned from the cache and playback starts immediately.

### 3.4 Viewer

The image viewer first shows the server's display image, and if conditions allow, fetches the original image from the content API and swaps it in once it has been decoded.
When to fetch the original image is chosen in the display preferences.
With automatic swapping, it is fetched right after the display image is shown.
With swapping only on zoom, it is fetched only when zoomed in finer than the display image.
The defaults of the display preferences are automatic swapping on a wide screen and swapping only on zoom on a narrow screen.

At either timing, what is swapped in is limited to images whose `metadata.contentType` is a format the browser can display (JPEG, PNG, GIF, WebP, AVIF, BMP).
Fetching right after the display image is shown in automatic swapping is further limited to images of 30 MB or less.
Images over 30 MB are, even in automatic swapping, fetched when zoomed in finer than the display image.
No size limit is set on fetching at zoom because the user is requesting the fetch by the zoom operation.
Images in formats that cannot be displayed, and images that failed to decode, continue to be displayed and zoomed as the display image.

For the display image's width and height, when the original image is fetched right after the display image is shown, the screen size (CSS px) is requested as is, and otherwise the value multiplied by the pixel density is requested.
For an image swapped in immediately, the display image is a stopgap until the original image arrives, and does not need to be made at high resolution.
Other images keep the display image until zoomed, so it is matched to the number of the screen's pixels.
The server rounds up to size steps, so the client need not know the step values.

Zoom is done with the wheel, a touch pinch, or a double tap.
A double tap switches between the whole image and an enlarged view, and the enlarged view counts as zooming for the timing of fetching the original image.
While zoomed, dragging moves the display position.

In the viewer, left and right flicks or key operations move to the previous or next Entry of the current listing (including search results).
A downward swipe closes the viewer, the same as the close button.
Flicks and the downward swipe are accepted only while the image is shown whole, so that they do not take over the drag of a zoomed image.
In the video viewer they are not accepted, so that they do not collide with the player's own controls.
The image area takes all touch gestures itself (`touch-action: none`, `overscroll-behavior: contain`), so the browser's pull-to-refresh and scrolling of the page do not start from it.
A flick is a horizontal movement of more than 50 CSS px within 300 ms, a downward swipe is a downward movement of more than 80 CSS px, and a double tap is two taps within 300 ms and 30 CSS px. A double tap at the whole view enlarges 2 times around the tapped point.
The order of previous and next follows the same sort as the listing.
When moving to an Entry on a page not yet read, that page is read.
The display images of one before and one after are requested in advance, so they are shown without waiting when moved to.
The shown image is wanted with the `interactive` request class and the ones before and after with `prefetch`, so the server starts the shown image first ([Image Channel Design §5](./image-channel.md#5-request-classes-and-order)).
The wants of images that are neither shown nor next to the shown image are dropped while loading. Besides the shown image and its neighbors, up to 6 more loaded display images are kept, the most recent first.
Moving to an image whose prefetch is still loading raises its want to `interactive`, so the server moves it ahead without starting it over.
The original images before and after are not requested in advance.
The targets of moving back and forth are the images and videos in the current listing, and Entries that are neither are skipped. Arrow keys pressed while focus is on the video player are left to the player.
When the destination is already known from back and forth movement, the filmstrip, or undo in rate mode, the viewer opens it at that position without waiting for the listing to be searched again, so neither "読み込み中…" (Loading…) nor a rebuilt filmstrip appears between Entries.
Opening the viewer pushes onto the URL history, and moving back and forth and closing do not.
The browser's back operation also closes the viewer, and even when a URL is opened directly with the viewer open, the same image appears.

The wide-screen image and video viewer has a filmstrip at the bottom edge.
The filmstrip assigns the same width to every Entry of the current listing (including search results), and lays them out horizontally in the same order as the listing.
The side of one position is chosen from 5 levels by the display preferences' filmstrip size: 40, 48, 56, 72, and 96 CSS px, and the section is 8 CSS px taller than the side. The default is level 3 (56 CSS px).
Changing the level measures the positions again and scrolls until the open Entry is at the center again.
At image and video positions it shows thumbnails, highlights the open Entry, and moves to the clicked Entry.
At the positions of other Entries it shows a faint icon indicating the kind and does not accept clicks.
While the open Entry is being searched for, the filmstrip stays at the previous position instead of being rebuilt.
The overall width is decided from `totalCount`, and only the pages of the range that became visible by scrolling horizontally are read (§3.5).
When the open image changes, it scrolls until that position comes to the center.
Thumbnails use the crop shape and are wanted under the same rules as the listing (§4), at the side times the pixel density, so changing the level changes the requested size.
Tab stops only at the open position, and distant positions are moved to by clicking.

The viewer has its own settings panel, a popover opened from a gear button in the header and drawn inside the outer frame element of `Modal`.
It holds the settings that only the viewer uses: the filmstrip size, on a wide screen only, and the swapping to the original image, for images only.
The gear is shown only when the panel would have a control: on a wide screen for an image or a video, and on a narrow screen for an image. A narrow video has neither setting, so it has no gear.
On a narrow screen the gear is next to the "その他" (More) menu, and on a wide screen it is with the other header buttons.
While the panel is open, the viewer's moving back and forth and closing with Esc are not accepted, so Esc closes only the panel and the slider's arrow keys change only the slider. The panel's Tab is left to the popover's own focus scope, and `Modal` does not move the focus ([Radix's nested layers](../knowledge/radix-nested-layer-events.md)).

The image viewer has a slideshow and full-screen display through the Fullscreen API.
The slideshow advances to the next image 5 seconds after the display image appears, and stops at the last image.
When moving back or forth midway, the 5 seconds are counted again from the image moved to.
Even for an image whose display image could not be read, it advances to the next once 5 seconds have passed since the failure was known.
The slideshow's automatic movement stops at videos as well as images. When it arrives at a video it starts playback automatically, does not advance in 5 seconds, and advances to the next after playing to the end. When it becomes known that playback is impossible, such as when the video's information cannot be read, it advances without stopping. It stops after the last video finishes playing. A slideshow can be started only from an image, and even while it is at a video, the operation to stop the slideshow is shown.
The wide-screen image and video viewer has rate mode, used to record ratings.
It is entered with the header button or `r`, and exited by pressing again. While in it, ← records bad and moves to the next Entry, → records good and moves to the next Entry, and ↓ records unrated (`none`) and moves to the next Entry. Rating and moving are done in one key operation, and it moves without waiting for the record to complete.
↑ undoes the last of these operations: it returns to the Entry that was rated and writes back the rating it had before. Undo goes back one operation at a time, newest first, and the history is kept only while the viewer is open. The rating before an operation is taken from the cache, or read from the server before the record is sent. If that read fails, the record is still sent, and undoing that operation fails with a toast. Records to the same Entry are sent in order, so an undo does not overtake the record it undoes.
During rate mode, four buttons laid out in a cross like the arrow keys are shown at the bottom right of the display area, and do the same as the keys. For a video they are raised above the player's controls.
The next Entry stops at videos as well as images. At the last Entry it does not move and only records the rating.
The open Entry's rating is shown beside the name. For an Entry that is not rated, a guide to the keys is shown. A saved rating is also shown when reopened.
It is not used at the same time as the slideshow, and starting one stops the other. When saving a rating fails, a toast notifies.
While a confirmation dialog and the "その他" (More) menu are open, key operations are not accepted. Rate mode is not shown on narrow screens.
Full-screen display targets the whole viewer, and in browsers that cannot use the Fullscreen API, the entrance is not shown.
Esc during full-screen display ends only the full-screen display, and does not close the viewer.
Closing the viewer also ends full-screen display.
On narrow screens, these two are placed in the "その他" (More) menu.

On a wide screen, downloading and deleting the open image can also be done from the viewer's buttons (§3.7).
Delete goes through the same confirmation dialog as the listing before being sent, and the slideshow is stopped at the moment the dialog is shown.
The result of deletion is notified by a toast over the viewer. The result of a deletion that closes the viewer is shown in a toast outside the viewer so that it remains visible after closing.
If the screen switches to narrow while a dialog is shown, the confirmation is canceled.
The viewer is modal, so the key operations of §3.7 do not work inside the viewer.
While a confirmation dialog, the "その他" (More) menu, and the viewer settings panel are open, moving back and forth and the key operation of closing the viewer with Esc are also not accepted.
After deletion completes, it moves to the next image, or the previous image if there is no next, or closes the viewer if neither exists.
While the next range is being loaded, delete cannot be chosen. While preparing and during the delete confirmation, the slideshow cannot be started. When the slideshow is stopped, automatic movement in the middle of loading is also canceled.
The destination is decided from the current listing before the confirmation dialog is shown. After deletion the listing is reread and positions shift, so it is held by relative path, not by position.

The video viewer receives media info from the server and judges whether it is a codec it can play.
The web client judges with `HTMLMediaElement.canPlayType`.
If it can play, it plays the content API directly, and if not, it creates a transcode session and plays HLS.
For browsers on the web client that cannot play HLS, hls.js is used.
When closing the viewer, it requests the end of the transcode session ([Video Playback Design §5.1](./video.md#51-transcode-sessions)).

### 3.5 Loading Listings

The web client's listing finds the overall number of rows from the first page's `totalCount`, and reserves that height in advance.
The position and length of the scrollbar correspond to the whole listing, not to what has been loaded.
Therefore, even if you jump to the end with the scrollbar, it can be displayed by reading only the page at that position.

Pages are requested 200 items at a time, by `offset`.
What is requested is only the pages containing the visible rows and the rows in the prefetch margin of one screen in the direction of scrolling.
At the position of pages not yet read, only the tile's frame is shown.
Requests for pages that have left the screen and also left the prefetch margin are canceled.

A page's response carries the snapshot's id ([Listing and Search Design §4.2](./listing.md#42-extracting-a-page)).
When a page arrives whose id differs from the pages already loaded, the listing is regarded as changed, the other pages are discarded, and the pages in the visible range are read again.
At this time the scroll position is kept, and the overall height changes by the amount `totalCount` changed.

When sorting, filtering, or the search terms are changed, the display returns to starting from the top of the listing.
The selection is held by Entry path. When a range selection (Shift-click and arrow keys, Ctrl or ⌘ plus A) spans pages not yet read, those pages are read first and the selection is made after reading finishes.
While loading, progress is shown, and it is canceled when the snapshot's id changes.
This is so that the selected count and the total size count all the Entries in the range.
A selection holds the Entry's path, and the size, the kind (file or folder), and the changeability at the time of selection. The total size can be counted regardless of whether it remains on a loaded page. The kind and changeability are held so that deletion and the like can decide the request's destination and the menu's state regardless of whether it remains on a loaded page.
When the snapshot's id changes and when the sort is changed, the selection is left as paths and only the start of the range is discarded. This is because positions change.
The selection is cleared when moving between folders, when switching between grid and list (§3.2), when the search changes (§3.6), and when the web client finishes changing Entries (§3.7).
When the web client itself performs a mutating operation, at the point it finishes (for an Operation, at the point it completes), that Library's listing and tree are reread. The displayed range is switched to the new snapshot while still showing the old contents.
The server also discards snapshots at the same point ([Listing and Search Design §4.3](./listing.md#43-freshness)), so the reread listing reflects the change.

The web client's tree displays from the first page of the expanded level, and additionally reads the pages needed for the visible range and expansion operations.
The rows it draws are limited to the visible range and the prefetch margin, and not all items are put in the DOM.
When a deep path whose position is unknown is explicitly opened, pages needed to display the ancestors are searched in order until found.
Unconfirmed nodes are displayed as expandable. Confirming whether there are children follows the rule of §5.1.

### 3.6 Search

Search is done in search mode, which is separate from folder view and the timeline. The URL is `/search`, and the queries are as follows.

| Query | Meaning |
| --- | --- |
| `lib` | A Library to search, repeated for each one. If omitted, every Library |
| `path` | The Library-relative path of the starting folder. Only when one Library is searched. If omitted, the whole Library |
| `flat=1` | Search only direct children of the start (not including subfolders). If omitted, searches underneath recursively |
| `q` | The search terms |
| `rating` | Filtering by rating. `good`, `bad`, `none` (unrated) |
| `tag` | Filtering by tag, repeated for each word. Entries with all the words match |
| `view` | The result open in the viewer, as the Library id followed by the Library-relative path with the leading `/` removed (§3.1) |

On a wide screen, the left area of search mode holds the search conditions in place of the tree. From the top, it lines up the field for search terms, a checkbox per Library, the start ("場所", Location), "サブフォルダを含めない" (Do not include subfolders), the rating choice (すべて (All), 良い (Good), 悪い (Bad), 未評価 (Unrated)), and a checkbox per tag word the user has created.
The grid or list, info panel, status bar, and viewer are the same as in folder view, and the toolbar shows no breadcrumb.
The last checked Library cannot be unchecked, so at least one Library is always searched. Checking every Library is the same as omitting `lib`, so a Library added later is also searched.
The start can be chosen only while one Library is checked. Pressing it opens a tree of that Library's folders and archives in a popover, and choosing a folder there makes it the start; the popover also has "Library 全体" (Whole Library). While several Libraries are checked, it reads "各 Library の全体" (Whole of each Library), and checking another Library returns the start to the whole Library.
When the search terms, rating, and tags are all absent, a listing is not requested and input is prompted.
Otherwise, `listSearchEntries` is requested with the checked Libraries, the start, the scope, and the conditions ([Listing and Search Design §5.3](./listing.md#53-searching-across-libraries)). When searching underneath, `recursive=true` is attached.
The results line up in the same grid and list as ordinary listings.

The switch's "検索" (Search) opens search mode with the conditions last used in search mode while the tab is open, or else every Library with no conditions. The result open in the viewer is not restored.
In folder view, pressing `/` outside input fields and dialogs on a wide screen, or choosing "このフォルダ内を検索" (Search in this folder) in the context menu of a tree folder or of empty space in the listing, opens search mode with that Library and folder as the start and no conditions.
In search mode, `/` focuses the search terms field. Rate mode (§3.4) can also be used in search mode's viewer, which allows narrowing to "未評価" (Unrated) and rating one after another.
Opening an old URL form replaces it with the corresponding `/search` URL. `/search/{libraryId}` becomes `lib` with that Library, its `tag` stays one word, and its `view`, relative to the start, is joined after the Library id and the start. `/browse/...` with `q` attached becomes that folder as the start, and those without `deep=1` correspond to `flat=1`.

While a search covers descendants or several Libraries, each Entry's parent folder is shown below the tile's name, in the "場所" (Location) column of a wide screen's list, and at the start of the second line of a narrow screen's list. A search of direct children in one Library shows no location.
While one Library is searched, it is a path relative to the start, and an Entry directly under the start is shown as ".". While several are searched, it is the Library name followed by the Library-relative path of the parent folder.
Opening a folder or archive result moves to that position in folder view, and the search ends. Moving by the switch also ends the search.
Opening an image or video result opens a viewer that moves back and forth among the search results.

On a wide screen, delete, download, cut, copy, rename, and "Show in folder" can be done, by the same rules as in the timeline (§3.10). A selection spanning Libraries is deleted with one request per Library, and cut and copy keep the Entries grouped by Library. There is no current folder, so new folder, paste, upload, and move by drag are not shown.
The info panel shows the Library name in addition to the Library-relative path.

On narrow screens, the field for search terms is always shown in a row below the search mode header, and the other conditions are in the left drawer. The drawer's button shows the number of conditions other than the terms that narrow the search.
When any condition changes, the selection is cleared. This is so that Entries that became invisible do not remain in the status bar's count and total size.
Entered terms are shown on screen immediately, and are put in the URL when they stop for 300 ms. The other conditions are put in the URL immediately. Changing conditions replaces the URL without pushing onto history. Entering search mode pushes onto history, so the browser's back operation can return to the screen of the entrance.
Terms that are only whitespace are treated as no terms and do not make a search.

When a rating or tags are specified, even without search terms, the images and videos under the start (directly under it, with "サブフォルダを含めない") that satisfy the conditions are listed. They can be combined with name terms, the Libraries, the start, and the scope, and narrow to those satisfying all of them ([Rating and Tag Design §7.1](./rating-tags.md#71-filtering-listings)).
A filtered listing is not reread when a rating or tag is given. The re-given result is reflected the next time the listing is opened and when the server returns a different identifier.

The first page of a subtree search does not return until the server's query of the name index, or its walk, finishes ([Listing and Search Design §5.2](./listing.md#52-answering-recursive-listings-with-conditions)).
In the meantime it indicates that searching is in progress, and the request is canceled when a condition is changed or the search is ended.
The server stops the work when all requests waiting for it have been cut ([Listing and Search Design §4.1](./listing.md#41-the-unit-that-is-built)).
When a subtree search from a folder outside archives gets a response telling that the name index did not answer, "索引を作成中のため、結果が不完全な場合があります。" (The index is being built, so the results may be incomplete.) is displayed above the results.
A search with "サブフォルダを含めない" or from inside an archive is walked by design and does not show it.
If the response's `truncatedLibraryIds` is not empty, it indicates that the result is partial and urges narrowing the Libraries or moving the start down and searching again.
A subtree search does not search inside archives, so to search inside an archive, choose the archive as the start.

Reload and the browser's back operation can return to the same search's results.

### 3.7 Entry Operations

Entry operations are done only on wide screens. When the screen switches to narrow, name dialogs and delete confirmations not yet sent are canceled. Requests already sent continue.
The entrances to operations are the context menu of tiles, list rows, tree items, and empty space in the listing, the toolbar, and key operations.
In `readonly` Libraries and inside archives, changing operations are made unselectable in the menu and toolbar ([design.md §8.6](../design.md#86-archives-and-readonly)).
Whether something can be changed is judged from the logical OR of the Library's `readonly` and the target Entry's `metadata.readonly`. For the destination of a new folder, it uses the metadata of the current folder obtained with `GET /folders`. While the metadata is being fetched and on failure, creating at the destination is made unselectable. If the server returns 403 for reasons such as the fetched value being stale, a toast notifies.

| Operation | Key operation | Request sent |
| --- | --- | --- |
| New folder | ⌥⇧N | Create a folder |
| Rename | F2 | A move into the same folder |
| Delete | ⌘⌫, Delete | Delete files and folders |
| Cut, copy, paste | ⌘X, ⌘C, ⌘V | move, copy |
| Upload | None | A one-shot write or an upload session |
| Download | None | The content API |

⌘ is read as Ctrl except on macOS, and ⌥ as Alt except on macOS.
The new-folder key is not ⌘⇧N because that is the key Chrome and Edge reserve to open an incognito window, and a web page cannot cancel it.

Rename and new folder are done in a modal dialog for entering a name. The dialog shows the target's Library-relative path for rename and the Library-relative path of the destination folder for a new folder, and places focus in the name input field. For renaming a file, it starts with the part up to the extension selected. Enter or "変更する" (Rename) / "作成する" (Create) sends the request, and Esc or "取り消す" (Cancel) closes it. When the name is empty, contains `/`, is `.` or `..`, or is unchanged in a rename, the send button cannot be pressed. When it fails, for example because an Entry of the same name exists, a toast notifies and the input can be corrected with the dialog left open. After creating, the listing is reread, and the created folder appears at its position in the order.
The dialog can be opened wherever the target is in the listing, so even if the target is outside the virtualization's rendered range, no scrolling or position search is done.
When a new folder, rename, or delete succeeds, the selection is cleared.
For a folder in the tree, new folder (created inside that folder), rename, and delete can be done from the menu, and the first two open the same dialog. Library root items do not show rename and delete.
Key operations are not made to work inside input fields and dialogs (including confirmation dialogs). Some environments change the character when ⌥ is pressed, so the new-folder key is judged by the physical position of the pressed key (`KeyN`).
Delete is confirmed with a dialog that shows the number of targets and that it cannot be undone, and then sent.
A deletion that includes folders states in the dialog that the contents are deleted along with them, and is sent with `recursive=true`.
A selection mixing files and folders is split into two requests by kind ([design.md §8.1](../design.md#81-namespaces)).

Cut and copy remember the selected Entries' Library and path in a clipboard inside the web client.
The OS clipboard is not used.
Paste targets the current folder or the folder that was right-clicked.
Cut cannot be chosen for a selection that includes read-only items. Copy can be chosen even for read-only ones.
Cut Entries are shown faintly until the paste request is accepted. When files and folders are sent as separate requests, only the items of the accepted requests are removed from the clipboard, and items rejected immediately are kept. Failures after acceptance, skips, and cancellations are notified in the activity panel, and the items to retry are chosen again.
Pasting across Libraries is also possible, and the move in that case becomes an Operation ([design.md §8.2](../design.md#82-determining-sync-and-async)).

Dragging selected Entries and dropping them on a tree folder or a folder tile moves them, and dropping while holding ⌥ (Alt) copies them.
Archives and `readonly` Libraries do not become drop targets. A drag that includes read-only items allows only copying with ⌥ (Alt) pressed.
During a drag, the listing scrolls at its top and bottom edges, and staying for 1 second over a closed folder in the tree opens that folder.

Uploads are done by dropping files or folders from the OS onto the listing or a folder tile, or from the toolbar's file or folder selection.
Files of 32 MiB or less use a one-shot write, and larger files use an upload session sending 16 MiB at a time ([design.md §8.7](../design.md#87-uploads)).
For a folder, the folder is first created at the destination, and then the files inside are sent.
At most 2 files are sent at the same time.

Download targets only one file, and opens the content API's URL as a link with the `download` attribute.
It is same-origin, so the session cookie is attached as is.

For paste, drop, and upload, before sending, it checks whether an Entry with the same name exists directly under the destination.
If the number of Entries to send is 50 or fewer, it requests the destination's listing with `query` for each name, and treats it as a conflict if there is an Entry whose name matches exactly.
If it exceeds 50, it reads the whole destination listing and compares.
Hidden items are also matched, and identical names among the Entries to send are also treated as conflicts. If matching the destination fails, nothing is sent.
If there are conflicts, it shows the number and names of the conflicts and asks only once among "置き換える" (Replace), "両方残す" (Keep both), "スキップ" (Skip), and "中止" (Abort).
The chosen policy (`overwrite`, `rename`, `skip`) is applied to all Entries when sending.
When the copy destination includes the source itself, only "両方残す" and "中止" are shown, and it is duplicated under another name. The original item is not overwritten. A move to the same location, and a move or copy to under the source folder itself, are rejected before sending.
If there are no conflicts, it is sent with the default `fail`.
A conflict that arose between the check and the send is reported as a failure by `fail`.

The result of an operation is notified by a toast for operations that return synchronously, and shown in the activity panel for operations that become Operations and for uploads.
The activity panel lists running and finished Operations and uploads, and shows each one's progress, the cancel operation, the skipped count, and the failed Entries and their reasons.
The state of running Operations is requested every second to advance it.
When an asynchronous operation starts, the activity panel is opened.
Items in the activity panel have a read state. When a collapsed panel is opened from the status bar button, the items lined up at that time become read and are shown faintly. The number in the button's parentheses counts only unread items, and is not shown when it is 0. Finished items can be removed from the list with the item's ×. Operations and uploads in progress are not made faint even when read, and cannot be removed until they finish. Read state and removal are kept only while the page is open, and on reload, the Operations reread from the server line up anew.

After reload, Operations are reread with `GET /operations` and returned to the activity panel.
Uploads cannot continue sending after a reload, because the browser holds the files being sent.
For that reason, if the page is about to be closed during an upload, the browser's confirmation is shown.

### 3.8 Info Panel

The info panel is placed to the right of the listing on a wide screen and shows the details of the selected Entry.
Its width can be changed with the handle at the left edge (§3.1), and it is opened and closed with a toolbar button.
The open/closed state is saved in the browser, and when opened for the first time it is in the open state (§5.1).
When nothing is selected, it guides that details appear when an Entry is selected.

When one Entry is selected, it shows a larger thumbnail, name, kind, size, modification date, and Library-relative path.
For folders and archives, it shows the representative thumbnail and the same items except size.
For files of kinds whose thumbnail cannot be shown, it shows the kind's icon.
For images and videos, it also shows media info.
For images, it shows the dimensions and shooting info (shooting date and time, camera, lens, focal length, aperture, exposure time, ISO sensitivity), and for videos, the duration, resolution, and codecs ([Video Playback Design §3](./video.md#3-media-info)).
The dimensions of an image are shown with the width and height swapped when the media info's `rotation` is 90 or 270 degrees.
For shooting info, it shows only the items Exif has, and shows no items when it has none.
The camera combines the maker name and the model name into one, and when the model name starts with the first word of the maker name, the maker name is not repeated.
When media info could not be read, a sentence says it could not be read.
When multiple Entries are selected, it shows the count and the total size.

The larger thumbnail and media info are requested only when the info panel is open and the selection has not changed for 200 ms.
When the selection changes, the want already sent is dropped.
This is so that, when moving the selection by clicking in succession, the server is not made to read ffprobe or Exif and generate large thumbnails for each Entry passed by.
The name, kind, size, and modification date are shown from the Entry the selection holds, without waiting for requests (§5.1).

When one image or video is selected, the rating and tags are shown below the media info.
The rating is display only, and giving it is done in the viewer's rate mode (§3.4).
For tags, the attached words are listed, and each can be removed with its ×. Entering a word in the input field and pressing Enter attaches it. While typing, the words created so far that are not attached are shown as completions, and a word not in the list is created as a new word as it is.
Each time a word is attached or detached, the Entry's whole tags are saved, and when saving fails, a toast notifies and the original words are kept. The Enter that confirms IME conversion does not attach.
Ratings and tags too are requested, like media info, only when the selection has not changed for 200 ms.
The rules for words and storage are authoritative in [Rating and Tag Design](./rating-tags.md).

### 3.9 Display Preferences

Display preferences are saved on the server.
The web client reads them at startup with `GET /preferences`, and writes the whole back with `PUT /preferences` each time they are changed.
If they have never been saved, the server returns the defaults.
Until reading finishes, the listing is not shown, and only the screen skeleton is shown.
If the listing were shown first with defaults, the tile size and shape would change after reading and thumbnails would have to be requested again.
When they could not be read, it runs with the defaults and notifies by a toast.
Even if the display preferences cannot be read, browsing is not stopped.
The shape and ranges of the set of values are defined by `openapi.yaml` and validated by the server.
A `PUT` that contains an out-of-range value is rejected with 400, and the saved values do not change.
`filmstripSize` is required. A set saved before it existed cannot be read, so `GET /preferences` fails with `SerdeError`, the web client runs with the defaults, and the next `PUT` saves the new shape. Deleting only the saved preferences row also resets it; the state directory holds users and ratings and is not deleted.
The ranges are written in the contract, so the generated router rejects when it reads the body. The body of this 400 is not an `ErrorResponse`, and the web client judges the rejection from the status code alone.

The display preferences have two sets: for wide screens and for narrow screens.
The web client decides which set to use by the screen width (768 CSS px), and switches sets when the width changes.
Only sorting does not depend on the screen width, so it is held once outside the sets.

| Value | Per set | Default (wide / narrow) |
| --- | --- | --- |
| Tile size (5 levels) | Yes | 3 (176 CSS px) / 3 (4 columns) |
| Thumbnail shape (crop, whole display) | Yes | Crop / Crop |
| Listing display (grid, list) | Yes | Grid / Grid |
| Swapping to the original image (automatic, only on zoom) | Yes | Automatic / Only on zoom |
| Filmstrip size (5 levels) | Yes | 3 (56 CSS px) / 3 (saved, not used) |
| Sorting (key and direction) | No | Name ascending |

The key and direction of sorting are the same as the values the listing API's `sort` and `order` take.
The values are changed in two places: the listing's values in the settings menu, a popover opened from the gear button at the right end of the toolbar, and the viewer's values in the viewer's settings panel (§3.4).
The menu shows the listing display, tile size, thumbnail shape of the set currently in use, and sorting, each under a heading, and directly changes those values.
The viewer's panel shows the filmstrip size and the swapping to the original image of the set currently in use.
The narrow set has a filmstrip size because the shape of a set is one, but the narrow screen has no filmstrip, so the value is saved and neither used nor edited.
The other set is changed from a screen of that width. The menu does not show which set it changes.
The menu shows only the choices of values that are in effect at that point.
When the listing display is the list, the tile size choice is not shown.
Making ineffective values selectable would show the user an operation that does nothing when changed.
Values that are not shown also round-trip as the saved values when written back.
Writing back is sent together when operations have stopped for 500 ms.
If writing back fails, the on-screen values are returned to the saved values and a toast notifies.
If writing back without rereading values another device saved first, the old whole that device had would be overwritten.
There is one owner, and situations of changing display preferences in succession from multiple devices are few, so detecting conflicts by version is not provided.

### 3.10 Timeline

Apart from folder view, the web client has a timeline that lists the images and videos of the chosen Libraries in one sequence in descending order of timeline date.
What is listed, the timeline dates, the order across Libraries, and the server's aggregation are authoritative in [Timeline Design](./timeline.md).

The timeline is placed at the URL `/timeline`, and carries the month being viewed in the `month` query (`YYYY-MM`) and the Entry open in the viewer in the `view` query.
The timeline has no current folder and spans Libraries, so `view` carries the Library id followed by the Library-relative path, with the leading `/` removed (`photos/2025/a.jpg`).
The old URL `/timeline/{libraryId}` is replaced by `/timeline`, keeping `month` and putting that Library in front of `view`.

Which Libraries to list is not put in the URL; it is saved in the display preferences (`timelineLibraries`).
While nothing is saved, every Library is listed. Saved ids of Libraries that no longer exist are ignored, and when none of the saved Libraries exists, every Library is listed.
Changing the choice rereads from the newest Entry and discards the selection.

The toolbar's switch moves from folder view to the timeline without changing the choice of Libraries.
Moving from the timeline or search mode to folder view opens the folder last opened in folder view on this device, or else the root of the first Library.

On a wide screen, instead of the tree, it lines up a checkbox per Library under the heading "Library" at the top, and below it, under the heading "年と月" (Years and months), the years and months with counts summed over the chosen Libraries.
The years and months take the same shape as the folder tree: rows have the tree's height, a year is a node at the first level that can be expanded and collapsed, and its months are at the second level, indented by one tree level.
The checkboxes sit in the column of the expand marks, so the Library names, years, and months line up with the tree's levels, and the counts line up at the right edge.
At first only the year of the month being viewed, or the newest year when there is none, is expanded. When the month being viewed moves into another year, that year is expanded too. Which years are expanded is kept only while the screen is open and is not saved.
The last chosen Library cannot be unchecked, so at least one Library is always listed.
Pressing a month rereads from that month's newest Entry. Pressing the month being displayed again also rereads.
The month being viewed by scrolling is highlighted, and `month` is put in the URL without pushing onto history.
On a narrow screen, this list goes in the same drawer as the tree.

The listing is the grid only, and the order is fixed to newest first.
Every time the day changes, a heading with the date and day of the week (2025年5月3日（土）, Saturday, May 3, 2025) is inserted and the row is broken.
Tile size and thumbnail shape use the same display preference values as folder view.
The settings menu does not show the switch between grid and list or sorting.

The timeline does not reserve the overall height in advance.
If the visible rows and the rows in the margin of one screen above and below have not yet been read, the continuation in that direction is read 200 items at a time.
The range read is held as one contiguous range, and Entries farther than 1000 items from the viewed position are discarded. When returning to the discarded side, it rereads from the cursor of the Entry at the discarded boundary.
When the continuation above is added and when distant Entries are discarded, the position of the Entry being viewed is kept. Even if row breaks change through reassembling rows, the row containing the Entry that was being viewed is found and the position is restored.
When a month is chosen and when opened from the URL's `month`, the range already read is discarded and reread from that month.
Thumbnails are wanted under the same rules as folder view (§4).

While the server is aggregating any of the chosen Libraries, "集計中" (Aggregating) and the number of files processed, summed over them, are shown, and the per-month counts are reread every 10 seconds.
Entries added by aggregation are not automatically inserted into the already loaded grid. Added Entries appear when "今すぐ更新" (Update now) is pressed and on the reread when a month is chosen again.
The "今すぐ更新" button makes the server start aggregation of the chosen Libraries.

On a wide screen, delete, download, cut, copy, and rename can be done from the tile's context menu, the toolbar, and key operations (§3.7).
There is no current folder, so new folder, paste, upload, and move by drag are not shown.
Changeability is decided per Entry from the `readonly` of its Library and of the Entry, and an operation is not offered when any selected Entry cannot be changed.
A selection spanning Libraries is deleted with one request per Library, and a failure in one Library does not stop the others.
Cut and copy keep the Entries grouped by Library, and pasting in folder view sends one group after another.
The info panel shows the Library name in addition to the Library-relative path.
Range selection is limited to the loaded range, and Ctrl or ⌘ plus A is not provided.
When it finishes changing Entries, the timelines that list a changed Library and their per-month counts are reread.
"Show in folder" opens the selected Entry's parent folder in folder view, selects that Entry, and scrolls to the display position.
For an Entry inside an archive, it opens the folder inside that archive.

The viewer moves to the previous and next images and videos in timeline order. The range the viewer holds also discards Entries farther than 1000 items from the open Entry and the filmstrip's range. The filmstrip does not read beyond 3000 items from the open Entry. This is to avoid holding everything between the open Entry and distant positions, since the range read is one contiguous range.
The filmstrip lines up the Entries of the range read and reads the continuation when scrolled to the end.
When opened directly from the URL's `view`, the range containing that Entry in the center is read.

The Entry of a file that vanished externally remains listed until the server's next aggregation.
That tile becomes an icon showing the kind, the same as files whose thumbnail cannot be made.

### 3.11 Languages

The web client is shown in English or Japanese. English is the default and the base of the translations.
When the user has not chosen a language, it follows the browser's language list in order and uses the first language it supports, or English if none matches.
The settings menu offers Auto, English, and 日本語. The choice is saved per device in the browser's `localStorage` under `pxna.language`, and Auto removes it.
The choice is read before the first render, so the screen does not flash in another language, and it also applies to the login and setup screens, which are shown before the server's display preferences are available.
Strings live in [en.ts](../../ui-web/src/i18n/locales/en.ts) and [ja.ts](../../ui-web/src/i18n/locales/ja.ts). The type checker requires `ja.ts` to have the same keys as `en.ts`, and a test requires the same placeholders.
Counts use plural keys: English has `_one` and `_other`, and Japanese has only `_other`. Dates and the names of months follow the language.
Strings are read with `t` at render time, so changing the language rebuilds the whole screen. Open menus and other transient state are lost, and server state stays in the `QueryClient`.
The `message` of an API error is English text for logs. The client shows a translation chosen by the error's `kind` and falls back to `message` only when the response has no `kind`.
The daemon and the API are not translated.

## 4. Thumbnail Requests

The client receives thumbnails, sprite sheets, and display images only through the image channel ([Image Channel Design](./image-channel.md)).
One image channel client per tab gathers the wants of every part: the grid, the list, the timeline, the filmstrip, flipbook playback, the info panel, and the viewer's display images.
Each part tells it which images it needs now, with a request class and an order within the class.
The channel client merges an image that several parts want, the same Library, path, kind, width, height, and shape, into one item with the highest class and the smallest order, and sends the changes of the merged set to the server.
The server decides from this set what to generate and send next ([Image Channel Design §5](./image-channel.md#5-request-classes-and-order)), so the client neither limits how many images it asks for at once nor cancels requests.

A listing wants the thumbnails of only the visible tiles and the tiles in the prefetch margin around them.
The prefetch margin is one screen in the direction of scrolling.
Visible tiles are `visible` and tiles in the margin are `prefetch`, and within each class tiles nearer the center of the screen get a smaller order.
When a tile leaves the screen and the prefetch margin, the listing stops wanting it, and the channel client drops the item unless another part still wants it.

The width and height requested are the tile's display size multiplied by the pixel density.
The server rounds up to size steps, so the client need not know the step values.
When the tile size or the thumbnail shape is changed, the visible tiles are wanted again with the new values.
On a wide screen, the tile's display size that is the basis of the requested size uses the minimum length of a side of the tile size level.
On a narrow screen, it uses the screen width divided by the number of columns.
The actual size after widening columns to fill the listing's width can be larger than the minimum.
For that difference, the obtained thumbnail is shown enlarged.
It does not want again merely because the listing's width changed and columns widened, and wants with the size at the time the folder was opened or the tile size was changed.
This is so that wants do not change in succession while the width is being changed.
The output format is not specified and is left to the server's configuration ([Thumbnail Design §4.2](./thumbnail.md#42-shape-and-output-format)).

The channel client keeps the images it receives in IndexedDB, by the same Library, path, kind, width, height, and shape, with the image's `ETag`.
When a part wants an image the cache holds, the channel client shows it at once and wants the item with that `ETag`. On `unchanged` it keeps the image, and on a new image it replaces the image in the cache and on screen ([Image Channel Design §7](./image-channel.md#7-revalidation)).
Thumbnails and sprite sheets have a budget of 256 MiB and display images one of 512 MiB, and each drops its least recently used images beyond the budget, so opening many display images does not push out the listing's thumbnails.
Logging out deletes the cache, because its images show the Library to whoever uses the browser next.
The channel client turns an image into a blob URL for the parts, and releases the URL when no part wants the image any more.

A tile whose item failed with the server's deadline expiry (`Timeout`) becomes an icon showing the kind, like files whose thumbnail cannot be made, and is not wanted again while that folder is displayed.
As with the failure of a listing, this is because repeating can hit the same deadline expiry.
When the channel closes, the channel client reconnects and sends its whole set again; items that were not answered are wanted again with new ids ([Image Channel Design §3](./image-channel.md#3-connection)).
When the server closes the channel because the session ended, the web client returns to login, and when it closes it for a contract error, the tiles that were not answered become icons.

The exception is right after the daemon's own change.
When the listing is reread after a change made through Pxna, the shown and failed states of folder and archive tiles are discarded, and the tiles in the request targets are wanted again with the `ETag` of the image the cache holds.
A representative can change with the contents, and the server compares the want with the representative chosen now, so it receives the new representative ([Thumbnail Design §10.1](./thumbnail.md#representative-revalidation)).
Image and video tiles are not wanted again, because their thumbnails change only when the file itself is replaced, and the next want revalidates them.

## 5. Design Decisions

### 5.1 Decided

#### The Web Client Combines a File Manager's Skeleton with a Photo App's Presentation

**Decision**
The web client puts a tree on the left, a grid or list in the center, a collapsible info panel on the right, and a status bar at the bottom, and puts the mode switch, the breadcrumb, display switches, and operations on the toolbar at the top (§3.1).
Tiles make large thumbnails the main attraction, and the viewer has a filmstrip.

**Reason**
In a desktop browser the screen is wide, and the folder hierarchy and contents can be shown at the same time.
Operations that walk the hierarchy and operations that view the contents can be moved between without screen transitions.
Pxna is used both for viewing the images and videos on a home server and for managing files, so it has both the skeleton needed for management and the presentation for viewing.

**Rejected alternative 1**
A method leaning toward a file manager such as Finder or Explorer, centered on dense display and management operations.
Management is easy, but as a screen for viewing images and videos the thumbnails are small and there is too much information, so we rejected it.

**Rejected alternative 2**
A method leaning toward a photo app, with minimal UI parts and the tree placed in a collapsible sidebar.
It suits viewing, but management operations come second and a Library with a deep hierarchy is hard to walk, so we rejected it.

#### The Web Client Targets Desktops and Provides Browsing on Narrow Screens

**Decision**
The web client is made for desktop browsers, and on narrow screens provides browsing, search, and the viewer, and does not show Entry operations (§3.1).

**Reason**
The Library can be viewed from a phone browser.
Entry operations presuppose multiple selection, drag and drop, the context menu, and key operations, so a separate touch operation system need not be built.

**Rejected alternative 1**
A method that targets only desktops.
One layout would suffice, but there would be no way to view the Library from a phone, so we rejected it.

**Rejected alternative 2**
A method that treats desktop and phone equally and makes the phone web client the main phone client.
It would make the phone web client a full client, but long-press menus, swipe operations, and consideration for transfer volume would have to be built on all screens, so we rejected it.

#### Show Archives in the Tree Too

**Decision**
The web client's tree also lists archives, with the same treatment as folders.

**Reason**
An archive is an Entry that can be entered like a folder, and if one can move into it from the grid but not from the tree, operations would change depending on the place.

#### Make Tiles Square and Switch Between Crop and Whole Display

**Decision**
Tiles are square, and whether to crop the thumbnail or fit the whole preserving the aspect ratio is chosen in the display preferences (§3.2).

**Reason**
Without changing the listing API, portrait or landscape images whose edges should not be cut can also be seen whole.
The tile sizes are uniform, so virtual scrolling can be done by row, and the row height can be decided before loading.

**Rejected alternative 1**
A method that packs rows while preserving the aspect ratio (justified).
It gives the look closest to a photo app, but the aspect ratio of each image is needed before drawing.
Putting dimensions in the listing response would require reading each file's header at every listing, ffprobe for videos, and extraction inside archives, so we rejected it.

**Rejected alternative 2**
A method with crop only.
It is the simplest, but the edges of portrait and landscape images are cut and the contents are hard to tell apart, so we rejected it.

#### Choose the Tile Size from 5 Levels

**Decision**
The size of tiles is chosen from the 5 levels of tile size, and switched with the slider, the wheel, and key operations (§3.2).
On a wide screen, the level's value is the minimum length of a tile's side, and columns are widened to fill the listing's width.

**Reason**
Thumbnails need to be requested again only when the level is changed, and requests do not go out in succession while the slider is being moved.
Five levels can cover from a small display for surveying the listing to a large display for comparing images one by one.
If columns are widened to fill the listing's width, no imbalance of margin remains between tiles and at the right edge regardless of the listing's width.

**Rejected alternative 1**
A method with a slider that changes the size continuously.
The operation is smooth, but a device to thin out requests while it is being moved would be needed, so we rejected it.

**Rejected alternative 2**
A method with three levels: small, medium, and large.
It is the simplest, but the difference between levels is large and it is hard to choose a column count that fits the screen width, so we rejected it.

**Rejected alternative 3**
A method that on a wide screen makes the level's value a fixed length of a tile's side.
The tile sizes would be uniform per level, but if the listing's width is not a multiple of the columns, a margin remains between tiles or at the right edge, so we rejected it.

#### Reserve the Listing's Overall Height in Advance and Read Only the Pages in the Visible Range

**Decision**
The web client's listing reserves the overall height with the first page's `totalCount`, and reads only the pages in the visible range and the prefetch margin by `offset` (§3.5).

**Reason**
Even for a large folder, it can be displayed by reading only the page jumped to with the scrollbar, with no wait to read the pages in between in order.
The scrollbar's length represents the whole listing, so one can tell roughly where one is.

**Rejected alternative**
Infinite scroll that reads the next page when nearing the end.
The previous implementation took this form, and it needs only to point at the continuation of a page with a cursor, but jumping to the end means reading all pages in order, and the scrollbar represents only the length of what has been loaded, so we rejected it.

#### When a Range Selection Spans Unread Pages, Read First and Then Select

**Decision**
When a range selection by Shift-click and arrow keys, or Ctrl or ⌘ plus A, spans pages not yet read, those pages are read first and the selection is made after reading finishes (§3.5).

**Reason**
In a listing that reserves the overall height in advance, for Entries on off-screen unread pages, neither the name nor the path is at hand.
The selection is held by Entry path, and the status bar's count and total size, and the delete, move, and copy to be added, use those paths.
If a range is selected after holding all its paths, the Entries one thought were selected do not go missing.

**Rejected alternative**
A method that selects only the loaded Entries.
The implementation is simple, but Ctrl or ⌘ plus A and Shift-click at a position jumped to by scrolling would select only some Entries.
It would cause the mistaken operation of deleting thinking everything was selected and having only some deleted, so we rejected it.

#### Separate Search into a Common Search Mode and Choose the Start Within Search Mode

**Decision**
Search is done in a search mode that belongs to neither folder view nor the timeline. The Libraries, the start (the whole Library, or under a folder), the scope (include descendants, or only direct children), and the conditions are chosen within search mode, and the entrances in folder view pass only the default start. The default scope includes descendants (§3.6).

**Reason**
If the search field were coupled to folder view, a search with the same experience could not be offered from another entrance, and the screen implementation would also be duplicated.
With a dedicated URL, reload and the back operation can return to the same search's results.
If the start can be chosen within search mode, a separate search need not be built for each entrance.
Filtering direct children gives the same result as a search with the scope set to direct children only.

**Rejected alternative 1**
A method that keeps the folder view's search field and adds a separate one to the timeline.
Adding an entrance is small, but there would be two search screens, and how the start is chosen and how results look would diverge, so we rejected it.

**Rejected alternative 2**
A method that narrows direct children while typing terms in the search field and goes to subtree search by a switch (the previous method).
Narrowing direct children is fast, but it presupposes search belonging to each entrance and cannot be made a common screen, so we rejected it.

#### Put the Entry Open in the Viewer in the URL as a Path Relative to the Current Folder

**Decision**
In folder view, the `view` query carries a path relative to the current folder, not the Entry's name. For an Entry directly under, it has the same value as the name (§3.1).
Search mode and the timeline have no current folder and span Libraries, so their `view` carries the Library id followed by the Library-relative path (§3.6, §3.10).

**Reason**
Search results and the timeline can include Entries of the same name in different folders and Libraries. With the name alone, the Entry to open from the URL cannot be decided as one.
With a relative path, the URLs of direct Entries in folder view do not change.

**Rejected alternative 1**
A method that carries the absolute value of the Library-relative path in `view`.
The Entry would be decided as one, but the form of the URLs of direct Entries would also change, and existing URLs would need to be reinterpreted, so we rejected it.

**Rejected alternative 2**
A method that carries only the name and takes the first of Entries with the same name.
The implementation is minimal, but it would open the wrong one of images of the same name in different folders (such as `IMG_0001.jpg`), so we rejected it.

#### Search on Narrow Screens Puts Only the Terms Field Under the Header

**Decision**
On narrow screens, the search terms field is always shown in a row under the search mode header, and the other conditions are placed in the left drawer, whose button shows how many of them narrow the search (§3.6).

**Reason**
The header on narrow screens is filled in width by the drawer button, the title, the switch, and the operations, and there is no room to place a search field.
The terms are what is changed most often while searching, so they stay at hand, while the Libraries, start, rating, and tags do not fit in one row and are changed less often.
The count on the drawer's button tells that the results are narrowed by conditions that are not visible.

**Rejected alternative 1**
A method that also puts the terms field in the drawer.
The screen would gain a row, but the drawer would have to be opened every time the terms are typed, so we rejected it.

**Rejected alternative 2**
A method that lines up every condition in rows under the header.
Nothing would be hidden, but the tags and Libraries would take several rows and push the results down on a phone screen, so we rejected it.

#### Make Name Input a Modal Dialog and Show the Target's Path

**Decision**
New folder and rename enter the name in the same modal dialog. The dialog shows the target's Library-relative path. When the screen switches to narrow, unsent dialogs and delete confirmations are canceled (§3.7).

**Reason**
The selection remains even when scrolled, and the listing is virtualized. With in-place editing, one needed to scroll to find a target outside the rendering range, and for a new folder to add a provisional tile outside the order.
With a dialog, it can be opened regardless of whether the target is rendered, and the implementation of listing virtualization and of editing can be kept separate.
Wherever it is opened from, in the listing or the tree, the same input and rules apply, and since the dialog can show the target's path, mix-ups are prevented.
Leaving input and confirmation on a narrow screen cannot keep the per-screen-width operations contract of browsing only, so they are canceled on switching.

**Rejected alternative 1**
A method that keeps in-place editing (the previous method).
The operation is light, but both the listing and the tree would need editing parts, and scrolling to targets outside the rendering range would be needed, so we rejected it.

**Rejected alternative 2**
A method where rename is in place and only new folder uses a dialog.
There would be two kinds of operation and the explanation of the rules would be doubled, so we rejected it.

#### Rate Mode Bundles Rating and Moving into ← → ↓, and Undoes with ↑

**Decision**
In rate mode, → is used for recording good and moving to the next Entry, ← for recording bad and moving to the next Entry, and ↓ for recording unrated and moving to the next Entry. ↑ undoes the last of these operations, returning to that Entry and writing back its previous rating. Recording does not make moving wait (§3.4). The same four operations are also buttons laid out in a cross.

**Reason**
Sorting is work of looking at images one by one and deciding, so if rating and moving to the next were separate operations, the operations would double in proportion to the number of images.
The left and right keys are assigned to the two-choice rating, and both advance to the next Entry. The ordinary meaning of ← → as "move back and forth" is replaced only during rate mode, so the direction of rating and the direction of moving do not coincide. A separate operation for entering is prepared so that it is not mistaken for ordinary movement.
By moving without waiting for the record, communication delay does not become a delay in consecutive operations.
Because every rating key advances, a wrong key moves past the Entry at once. Undo returns to it and restores the rating in one operation, without leaving rate mode.
The buttons mirror the arrow keys so that the assignment can be read from the screen, and rating can be done with the mouse alone.

**Rejected alternative 1**
A method that puts the rating keys at another position such as number keys and leaves moving to the existing keys.
The inputs would not be mixed, but two operations would be needed for rating and moving, so we rejected it.

**Rejected alternative 2**
A method where ↑ moves to the next Entry without rating (the previous method).
A mistake could only be fixed by leaving rate mode, moving back, and rating again, so we rejected it.

**Rejected alternative 3**
A method where undo only returns to the previous Entry without writing back its rating.
The wrong rating would stay unless rated again, and an Entry that should be unrated could not be restored without another key, so we rejected it.

#### Use shadcn/ui for Parts Such as Menus and Dialogs

**Decision**
shadcn/ui is used for menus, context menus, dialogs, toasts, popovers, sliders, and tooltips.
shadcn/ui is used in the form of bringing into the repository the code of parts that wrap Radix primitives with Tailwind.
The tree, grid, list, and viewer are built in-house.

**Reason**
Entry operations need context menus, confirmation dialogs, notifications, and nested menus, and writing key operations, focus management, and position adjustment at screen edges in-house would be the largest amount of implementation.
shadcn/ui places the parts' code in the repository, so look and behavior can be fixed directly, and Tailwind's way of writing can be used as is.
It is not used for virtualized parts, so the part library's internal virtualization and TanStack Virtual do not collide.
The viewer has its own full-screen key operations and drawing, so it is opened as the browser's modal as is.

**Rejected alternative 1**
A method that keeps building all the parts in-house.
The previous implementation followed this policy, and dependencies do not increase, but menu key operations, focus management, and position adjustment would be written in-house, so we rejected it.

**Rejected alternative 2**
A method based on Base UI made by MUI.
The usage is the same as shadcn/ui, but it is newer than Radix and there are fewer examples to refer to when assembling parts, so we rejected it.

#### Switch the Color Scheme by Looking Directly at prefers-color-scheme

**Decision**
The web client's color scheme follows the OS's light and dark setting with a small function that looks at `prefers-color-scheme` and toggles the `dark` class of `<html>` (§3.1).

**Reason**
What it follows is only the OS setting, and there is no switch the user chooses on screen.
So a theme part with a storage location and a switching mechanism is not needed.
If `next-themes`'s `ThemeProvider` is used, React 19 prints an error to the console that the `script` element it renders is not executed.

**Rejected alternative**
A method that switches with `next-themes`'s `ThemeProvider`.
It would be convenient if the user is later allowed to choose light and dark, but for now it only emits the error above and gains nothing, so we rejected it.

#### Do Move and Copy with the Clipboard and Drag and Drop

**Decision**
Move and copy are done with cut, copy, and paste using a clipboard inside the web client, and with drag and drop to the tree or folder tiles (§3.7).

**Reason**
They can be used with the same operations as Finder or Explorer, and if the destination folder is visible on screen, one drop suffices.
For a distant folder, cut, then open that folder and paste.

**Rejected alternative 1**
A method with only cut, copy, and paste.
It avoids handling drag in the virtualized listing and tree (auto-scroll at the edges, auto-expansion of the tree), but even a move to a visible folder would need three operations, so we rejected it.

**Rejected alternative 2**
A method that opens a dialog for choosing the destination with "Move" and "Copy".
It completes without moving to another screen, but another tree would have to be built inside the dialog, so we rejected it.

#### Check Name Conflicts Before Sending and Ask Only Once

**Decision**
For paste, drop, and upload, whether an Entry of the same name exists directly under the destination is checked before sending, and if there are conflicts, the policy is asked only once and applied to all Entries (§3.7).

**Reason**
When there are no conflicts nothing needs to be asked, and even with conflicts, one choice lets it be sent as a single operation.
A conflict arising between the check and the send is caught by the default `fail` as a failure.

**Rejected alternative 1**
A method that first sends with `fail`, and has only the failed Entries choose a policy and be sent again.
There would be no race between check and send, but one copy would be split into two operations and the display of results would also split in two, so we rejected it.

**Rejected alternative 2**
A method that has the policy for conflicts chosen first at every paste.
The implementation is simple, but one more operation would be added even when there are no conflicts, so we rejected it.

#### Keep Each Request's Result in Mixed Moves and Copies

**Decision**
A selection mixing files and folders is split into requests by kind, and both responses are awaited. The synchronous results and the accepted Operations are kept, and the failure of one does not discard the result of the other. Cut removes only the items accepted per request and keeps the items rejected immediately. If another cut or copy is performed during the operation, that new content is not changed by the response of the earlier request.

**Reason**
The API's endpoints are split by kind, and a mixed selection cannot be sent in a single request. Even if partially successful, the user needs to be able to confirm the remaining failures and the accepted operations. Keeping accepted items allows resending, and emptying the whole would require reselecting the items rejected immediately. Individual failures after acceptance are checked in the activity panel, and the items to retry are chosen again.

**Rejected alternative**
A method that waits for the Operation's completion and restores the items not moved. It would need additional processing to reconcile partial failures inside folders with the top-level sources, so we rejected it.

#### Copy to the Same Place Is Limited to Duplication Under Another Name

**Decision**
When the copy destination includes the source itself, it is confirmed with "両方残す" (Keep both) and "中止" (Abort). Only duplication under another name is allowed, and the original item is not overwritten. A move to the same place, and a move or copy to under the source folder itself, are rejected.

**Reason**
Pasting into the same folder can duplicate, while the deletion of the original item and recursive copy into oneself must be prevented.

#### Apply the Conflict Policy to Bringing In Folders Too

**Decision**
The top-level name being brought in is matched only once, and the chosen policy is also applied underneath. Creating a folder has no conflict, so the web client decides the creation destination. `overwrite` merges into an existing folder, and if there is a file of the same name, deletes it and creates the folder. `rename` creates it under an available other name and uses that name for the destinations underneath. `skip` does not send the existing folder or what is underneath, and shows skipped in the activity. `fail` and a conflict at creation time are notified as failures.
Empty folders are also brought in, and the creation result is shown in the activity. If canceled during creating a session, it discards after receiving the identifier, and after sending a commit it waits for the response and notifies the publication result. Folders already created and files already sent are not rolled back on failure or cancellation midway.

**Reason**
Even if a folder is renamed, files underneath should not be sent into a folder of the original name, and the chosen conflict policy must be kept across the whole bring-in. The API accepts conflict only for publishing files, so the client supplements creating folders.

#### Keep Each Request's Result in Mixed Deletions

**Decision**
Deleting files and folders waits for both responses, and even if one fails, the successful request's Operation is handed to the activity panel. For what was deleted synchronously, the listing is reread, and the failed request is notified by a toast.

**Reason**
There is no mechanism to cancel two requests together. Reporting only the failure of one would leave the user unable to confirm the progress or result of the other.

**Rejected alternative**
A method that discards the results on the first failure loses sight of Operations already accepted, so we rejected it.

#### The Slideshow Plays a Video to the End Before Advancing

**Decision**
The slideshow's automatic movement also stops at videos, and a video is played to the end before advancing to the next (§3.4).

**Reason**
If videos were skipped, a folder mixing images and videos could not be viewed as a flow.
Videos vary in length, so cutting at the same 5 seconds as images would advance to the next without it being seen.

**Rejected alternative 1**
A method that skips videos and flows only images (the previous method).
The implementation is small, but in folders containing videos, only the videos would need to be opened by hand, so we rejected it.

**Rejected alternative 2**
A method that advances videos to the next in 5 seconds too.
They could not be seen to the end, which does not suit the slideshow's purpose, so we rejected it.

#### Gather Display Preferences into a Settings Menu and a Viewer Panel

**Decision**
The listing display (grid and list), tile size, thumbnail shape, and sorting are placed in a single settings menu opened from the toolbar's gear button, on both wide and narrow screens. The menu changes only the set currently in use (§3.9).
The settings that only the viewer uses, the filmstrip size and the swapping to the original image, are placed in the viewer's settings panel (§3.4), not in the settings menu.

**Reason**
When operations line up on the toolbar, they compete for width with the mode switch, the breadcrumb, and the operations. Display settings are not changed often, so folding them into a menu is not a problem.
Wide and narrow screens use the same part, so implementations per screen width need not be separated.
A separate view menu and settings screen showed the same values in two places, and the user had to know which entrance held which value.
The swapping to the original image and the filmstrip size only matter while the viewer is open, and the viewer is modal, so the listing's menu cannot be reached from it. Placing them in the viewer lets the user see the effect of a change while changing it.
Each value still has one entrance.

**Rejected alternative 1**
A method that lines them up on the bar on wide screens and makes a menu only on narrow screens.
Operation would take one step, but the bar gets crowded and the parts are kept in two ways, so we rejected it.

**Rejected alternative 2**
A method that keeps, next to the menu, a settings screen that shows the wide and narrow sets side by side (the previous method).
The other set could be changed from any device, but the menu and the screen overlapped in most values, so we rejected it.

#### Change the Width of the Tree and Info Panel by Dragging, and Save It per Device

**Decision**
The tree and the info panel change width by moving the handle at the border with the listing by dragging or keys, and the width is saved per device in the browser's `localStorage`. Values beyond the range are clamped into the range. The timeline's list of months is not changed (§3.1).

**Reason**
The suitable width changes with the depth of the Library's hierarchy, the length of names, and the screen size.
Like the open/closed state, it is a per-device way of viewing, so it is saved in the browser, not in the server's display preferences.

**Rejected alternative**
A method that saves the width in the server's display preferences.
It would be consistent across devices, but the same width does not suit devices with different screen sizes, so we rejected it.

#### Choose the Language by the Browser and Save the Choice in the Browser

**Decision**
The language follows the browser's language list unless the user chooses one, and the choice is saved per device in `localStorage` (§3.11). The daemon and the API are not translated; the client translates API errors by `kind`.

**Reason**
The language is a per-device way of viewing, and the browser's language already differs per device.
`localStorage` can be read synchronously before the first render and works on screens shown before sign-in, which the server's display preferences cannot.
Translating in the client keeps the daemon and the API contract unchanged.

**Rejected alternative 1**
A method that saves the language in the server's display preferences.
It would follow the user across devices, but it needs an API change, cannot be read before sign-in, and delays the first render.

**Rejected alternative 2**
A method where the daemon translates error messages by `Accept-Language`.
It would need internationalization on the Rust side for messages that are mostly shown as short notices, so we rejected it.

#### Activity Becomes Read When Opened, and Finished Items Can Be Removed

**Decision**
When the panel is opened, the items lined up at that time become read and are shown faintly, and only the number of unread ones is shown in the status bar's button. Finished items can be removed with ×, and items in progress remain until they finish (§3.7).

**Reason**
If finished items stay lined up, they cannot be told apart from new results and the number on the button does not decrease.
If items in progress could be removed, the cancel operation would be lost.

**Rejected alternative**
A method that removes them automatically after a fixed time.
They would vanish before being read, so we rejected it.

#### Show the Progress of Asynchronous Operations and Uploads in the Activity Panel

**Decision**
Operations that become Operations and uploads are lined up in the collapsible activity panel at the bottom right, showing progress, cancellation, and failed Entries (§3.7).

**Reason**
The progress of large copies and uploads is visible, and they can be canceled midway.
Even after a reload, the state can be restored from the Operations remaining on the server.

**Rejected alternative 1**
A method that only notifies start, completion, and failure by toasts.
It is enough for small operations, but the progress of large copies and uploads is unknown and they cannot be canceled, so we rejected it.

**Rejected alternative 2**
A method that shows the number running in the status bar and opens details on click.
It does not occupy the screen, but with the number alone one cannot know which operation has progressed how far and which Entries failed until opening it, so we rejected it.

#### Save Display Preferences on the Server and Hold Two Sets by Screen Width

**Decision**
Display preferences are saved on the server as a set of values whose shape and ranges are defined in `openapi.yaml`, and hold two sets, for wide screens and narrow screens (§3.9).

**Reason**
From whichever device they are opened, the same kind of screen gets the same display.
The suitable values for tile size and swapping to the original image differ with the screen width and the connection, so one set cannot match each kind of device.
The shape and ranges are placed in the API's contract, so the server can validate values.
The cost is that every time a value is added, a change to the contract and the server is needed.

**Rejected alternative 1**
A method that saves to localStorage per browser.
No server change is needed, but values would not be consistent across devices, so we rejected it.

**Rejected alternative 2**
A method that saves only one set, and on narrow screens the web client applies fixed rules (such as the original image only on zoom).
The mechanism is the simplest, but the user cannot choose the values for narrow screens, so we rejected it.

**Rejected alternative 3**
A method that takes the server's values as defaults and allows overriding per device with localStorage.
It is flexible, but it is hard for the user to see which values are in effect, so we rejected it.

**Rejected alternative 4**
A method in which the server saves JSON as is per kind of client without validating the contents.
Only the web client would need to change to add values, but the web client would hold the migration and validation of the shape and could not share the definition with other clients, so we rejected it.

#### The Viewer First Shows the Display Image

**Decision**
The image viewer first shows the display image the server shrank (§3.4).
By the wide-screen default, it then fetches the original image, and if it is in a format the browser can display and is 30 MB or less, swaps it in once it has been decoded.
Images over 30 MB are fetched when zoomed in.
By the narrow-screen default, the original image is fetched only when zoomed in.
Which to use can be chosen in the display preferences per screen width.

**Reason**
The display image is small, and the wait right after opening is short.
Formats the client cannot display can also be seen through the display image the server converted.
Desktops are often used on LAN connections, the transfer of several MB for an original image finishes soon, and detail lost to shrinking and compression of the display image can be seen as it is.
On a phone, fetching the original photo as is would mean waiting for a transfer of several MB per image.
To decide what to swap in, the web client holds the list of formats the browser can display.
This list is not the formats the server can read but the list of the web's standard image formats, and does not change even when the server's support grows.

**Rejected alternative 1**
A method that fetches the original image from the start.
The server's work would not increase, but displaying large photos would be slow and formats that cannot be displayed would remain, so we rejected it.

**Rejected alternative 2**
A method that fetches the original image, and fetches a converted image only when the format is one the client cannot display.
The long wait right after opening a large photo would remain, so we rejected it.

**Rejected alternative 3**
A method that fetches the original image only on zoom and raises the size steps and quality of the display image.
Behavior would be the same regardless of format, but the display image never becomes the same as the original image, and the server's conversion load and cache capacity would increase, so we rejected it.

**Rejected alternative 4**
A method that switches to the original image with a viewer button.
The user could decide the transfer volume, but an operation would be needed every time a photo is flipped, so we rejected it.

#### Gather Every Image Want of a Tab in One Image Channel Client

**Decision**
One image channel client per tab merges the image wants of every part and sends them on one image channel (§4).

**Reason**
The listing, the filmstrip, the list, the info panel, the timeline, and the viewer each had their own queue, limited to 4 requests, and none saw what the others asked for. The same image was asked for in two sizes by the listing and the filmstrip, and the server decoded it twice, and the queues and the display images shared the browser's six connections to the daemon ([Image Channel Design §9.1](./image-channel.md#91-decided)).
With one client, the same image wanted by two parts is one item, and the server receives every want of the tab and merges them by source.

**Rejected alternative**
Letting each part open its own channel. The parts would keep their own state, but the server would see several connections and the tab would hold several sockets.

#### Keep Received Images in IndexedDB and Revalidate Them

**Decision**
The channel client keeps received images in IndexedDB with their `ETag`, with separate budgets for thumbnails and sprite sheets (256 MiB) and display images (512 MiB), drops the least recently used beyond the budget, revalidates each kept image when it is wanted, and deletes the cache on logout (§4).

**Reason**
The browser's HTTP cache does not keep what a WebSocket carries. Without a cache of its own, every reload and every new tab would transfer every image again. IndexedDB holds blobs with the fields needed to find and evict them, which the Cache API, keyed by request, does not.
Separate budgets keep the larger display images from pushing out the listing's thumbnails.

**Rejected alternative 1**
One budget for every kind. It is simpler, but opening many display images in the viewer would evict the listing's thumbnails.

**Rejected alternative 2**
Leaving eviction to the browser's storage quota. It needs no code, but the cache could take much of the device's storage, and what is evicted could not be controlled.

#### Line Up All the Listing's Entries in the Filmstrip

**Decision**
The filmstrip assigns the same width to every Entry of the current listing, lays them out in the same order as the listing, and reads only the pages of the range that became visible (§3.4). The width is the same for every Entry and is chosen from 5 levels by the filmstrip size.
Images and videos show thumbnails and can be moved to by clicking, and at the positions of other Entries, an icon showing the kind is shown.

**Reason**
The listing API cannot filter by kind (§5.2's "Whether to filter listings by kind"), and `totalCount` counts all Entries.
How many images there are and where they are in the listing is not known until all pages are read.
If the filmstrip's positions are aligned with the listing's order, the overall width is decided from `totalCount` alone, and as with the listing, it can be displayed by reading only the page jumped to by scrolling (§3.5).
The width is not varied by Entry kind because the positions of Entries on unread pages are computed without knowing their kind.
Other Entries are not made targets for moving by click, to align with the rule limiting the targets of moving back and forth to images and videos (§3.4).
The cost is that in ranges where other Entries continue, only icons line up.
The number of images and videos is not known, so it also does not indicate which number the open Entry is.

**Rejected alternative 1**
A method that places the open image in the center and lines up only a few images before and after.
Picking from the loaded pages suffices and the cost on opening does not depend on the listing's size, but the only means of moving to a distant image would be repeating back and forth movement, so we rejected it.

**Rejected alternative 2**
A method that reads all pages when the viewer opens and lines up only images.
Only images would line up and which number could be shown, but for a listing of 50,000 items each opening would issue 250 requests, so we rejected it.

#### The Web Client Represents the Current Position with the URL Path

**Decision**
The web client puts the current Library and Library-relative path in the URL path, and the Entry open in the viewer and the search terms in the query (§3.1).

**Reason**
Reload, bookmarks, and the browser's back operation can return to the same place.
Represented by path, the URL has the same shape as the Library's hierarchy, and the place is understood by reading it.
The daemon returns `index.html` for the web client's routes ([design.md §10.1](../design.md#101-web)), so representing by path needs no addition on the serving side.

**Rejected alternative 1**
A method that represents the position only by query string.
The daemon's fallback to `index.html` would not be needed, but the Library-relative path would become one percent-encoded value, and it would be hard to read the place from the URL, so we rejected it.

**Rejected alternative 2**
A method that does not put the position in the URL and holds it only in the screen's state.
No router would be needed, but on reload it would return to choosing a Library, and the back operation could not return to the previous folder, so we rejected it.

#### Generate the Web Client's API Types from openapi.yaml

**Decision**
The web client calls the API with types generated from `openapi.yaml` by openapi-typescript and openapi-fetch, which uses them.
The generated types are placed in the repository, and CI checks that there is no diff against the regenerated result.

**Reason**
As with the daemon, `openapi.yaml` is kept as the authoritative contract, and discrepancies between the contract and the web client can be detected by type checking and CI.
Only types are generated, and the runtime processing remains a thin wrapper around `fetch`.
Passing an `AbortSignal` to cancel a request and receiving responses as blobs can also be done as they are.

**Rejected alternative 1**
A method that generates a TypeScript API client with the same openapi-generator as the daemon.
The generator version could be aligned, but the output is large and class-centered, and the hand-written routes (file content) would end up being called directly with `fetch` anyway, so we rejected it.

**Rejected alternative 2**
A method that generates even TanStack Query hooks (orval).
The description of calls would shrink, but the generated output and its configuration would grow, and hand-written routes could not be handled in the same form, so we rejected it.

**Rejected alternative 3**
A method of writing the types and calls by hand.
Dependencies would not increase, but discrepancies with `openapi.yaml` could not be detected mechanically, so we rejected it.

#### Use TanStack Parts for the Web Client's State, Routing, and Virtual Scrolling

**Decision**
The web client holds data fetched from the server with TanStack Query, the correspondence between URLs and screens with TanStack Router, and the listing's virtual scrolling with TanStack Virtual.
The look is written with Tailwind, and the parts and key operations of the tree, grid, list, and viewer are built in-house.
Non-virtualized parts use shadcn/ui ("Use shadcn/ui for Parts Such as Menus and Dialogs" in this section).

**Reason**
TanStack Query handles per-folder listing caches, loading pages, sharing the same listing between the tree and the listing, and discarding everything at once on a 401, with ready-made mechanisms.
TanStack Router can check URL paths and queries with types.
TanStack Virtual returns the displayed range and scroll direction as they are, so the values passed to the image wants (§4) and page requests (§3.5) are easy to assemble.
We build the tree, grid, and list in-house so that, combined with virtual scrolling, they do not collide with the virtualization that a part library holds internally.

**Rejected alternative 1**
A method of holding server data with SWR.
It is smaller, but handling of mutating requests and the discarding after them, and cancellation of requests, is weaker than TanStack Query, so we rejected it.

**Rejected alternative 2**
A method of handling URLs with React Router.
It has more users and information, but we would have to write the typing of queries ourselves, so we rejected it.

**Rejected alternative 3**
A method that virtualizes the grid with react-virtuoso.
It even provides additional page loading, but internal state such as scroll direction is hard to extract, and the values to pass to the image wants are hard to assemble, so we rejected it.

**Rejected alternative 4**
A method that builds the tree and dialogs with React Aria Components.
Accessibility and key operations could be delegated, but it has its own virtualization and is hard to use together with TanStack Virtual, so we rejected it.

#### Open the Web Client's Viewer as a Browser Modal

**Decision**
The web client's viewer is opened with `dialog.showModal()` and makes the background inoperable.
Tab and Shift+Tab cycle among valid operable elements, and after closing, focus is returned to the element from before opening if it remains.

**Reason**
If focus is on the background, the same arrow keys move both the listing and the viewer.
By using the browser's modal mechanism and handling the focus cycle and restoration in a shared part, the same key operations are kept for images and videos.

#### Draw the Viewer's Confirmation Dialogs, Menus, and Toasts Inside the Outer Frame Element

**Decision**
The viewer's confirmation dialogs, menus, and result toasts are drawn inside the outer frame element within the modal `dialog`.
The target of full-screen display is also made that outer frame element.
For the viewer's result toasts, a dedicated `Toaster` is placed inside the outer frame element.

**Reason**
While the browser's modal is open, elements drawn outside the `dialog` end up behind it and cannot be operated.
The `dialog` element itself cannot be made full screen, but an element that is its descendant can, and elements drawn inside it are also visible and operable in full screen ([Measurements of Chromium](../knowledge/chromium-modal-dialog-fullscreen.md)).
If aligned to the outer frame element, the header, filmstrip, confirmation dialogs, menus, and toasts are seen in the same place in both modal and full screen.
In the confirmation dialogs and menus inside the outer frame, the Esc `keydown` reaches the viewer even after Radix handles it, and a Tab reaches the `Modal` cycle, so the viewer does not handle a `keydown` that has already been handled, and `Modal` does not handle a Tab inside ([Measurements of Radix](../knowledge/radix-nested-layer-events.md)).

**Rejected alternative 1**
A method that leaves the confirmation dialogs and menus drawn directly under `body`.
No change to the parts would be needed, but they would end up behind the viewer and could not be operated, so we rejected it.

**Rejected alternative 2**
A method that makes the modal `dialog` element itself full screen.
It would not need an added outer frame element, but the browser refuses it, so we rejected it.

#### The Info Panel Shows the Entry the Selection Holds and Requests Only Media Info

**Decision**
The selection holds a reference to the selected Entry.
The info panel shows the name, kind, size, modification date, and Library-relative path from that Entry, and requests only media info and the larger thumbnail (§3.8).

**Reason**
The selection holds the size and such at the time of selection, keyed by path (§3.5).
To look up an Entry from a path, one would have to search the listing's pages and check whether they are loaded, and when pages are discarded by a snapshot change, only the selection would remain.
If the selection holds a reference, items can be shown right after selecting, without adding requests, regardless of the state of the listing's data.
The Entry reference points to the same value as the listing's cache, so a copy of the Entry is not made for the selection.

**Rejected alternative 1**
A method that requests and shows the file's details from the selected path.
It can be obtained independently of the listing, but a request would go out each time one selects, and the display of items would wait for the request to complete, so we rejected it.

**Rejected alternative 2**
A method that looks up and shows the Entry of the selected path from the listing's data.
Requests would not increase, but the selection holds no position, so it would search the loaded pages exhaustively, and when pages are discarded by a snapshot change it would not be found, so we rejected it.

#### Save the Info Panel's Open/Closed State in the Browser

**Decision**
The info panel's open/closed state is saved in the browser's `localStorage`.
When opened for the first time, it is in the open state.

**Reason**
Whether one wants to keep it open differs with the device's screen size and way of use, and does not need to match the server's values.
Opening and closing is a preference for how to view the listing, and even if saving fails, browsing is not hindered.

**Rejected alternative 1**
A method that adds it to the display preferences and saves it on the server.
It would be consistent across devices, but it would need changes to the display preferences API and generated artifacts, and which of the wide and narrow sets to put it in would also have to be decided, so we rejected it.

**Rejected alternative 2**
A method that does not save and returns to the open state at every reload.
The implementation is the simplest, but a user who wants to use it collapsed would have to collapse it again at every reload, so we rejected it.

#### Put the Timeline on a Separate Route and Switch with the Toolbar

**Decision**
The timeline is placed at `/timeline`, and the toolbar's switch goes back and forth with folder view and search mode (§3.10).

**Reason**
Reload, bookmarks, and the browser's back operation can return to the same month of the timeline.
The timeline has no current folder, so the URL can omit a folder path.

**Rejected alternative 1**
A method that adds a query to the folder view's URL to indicate the timeline.
Routes would not increase, but a folder path that the timeline does not use would remain in the URL, and the same timeline would become as many different URLs as there are paths, so we rejected it.

**Rejected alternative 2**
A method that saves in the display preferences whether to open in folder view or the timeline.
It would be convenient for a user who always views the timeline, but the display could not be known from the URL, and the results of shared URLs and the back operation would change with the settings, so we rejected it.

#### Save the Timeline's Libraries in the Display Preferences as the Chosen Ones

**Decision**
The Libraries the timeline lists are chosen with checkboxes in the timeline's left area and saved in the display preferences as the list of chosen Libraries, not put in the URL (§3.10).
While nothing is saved, every Library is listed.

**Reason**
Which Libraries hold photos rarely changes, so the choice is made once and kept across reloads and devices.
With the choice only in the preferences, there is no state in which the URL and the saved value disagree, and Pxna has a single owner, so a URL that reproduces someone else's choice is not needed.
Saving the chosen Libraries rather than the excluded ones keeps a newly added Library out of the timeline until it is chosen, so a Library of other images does not get mixed in unnoticed.
The cost is that a newly added Library of photos has to be checked once.

**Rejected alternative 1**
A method that carries the chosen Libraries in the URL query and uses the saved value only when the query is absent.
A narrowed timeline could be bookmarked, but which of the URL and the saved value wins, and when the saved value is updated, would have to be decided, so we rejected it.

**Rejected alternative 2**
A method that saves the excluded Libraries.
A newly added Library would appear without any operation, but a Library of other images would also be mixed in until excluded, so we rejected it.

#### Line Up Years and Months in the Timeline's Left Area and Jump to the Chosen Month

**Decision**
In the timeline, instead of the tree, years and months are lined up with counts, and pressing a month rereads from that month (§3.10).

**Reason**
The timeline does not reserve the overall height, so one cannot jump to distant periods with the scrollbar.
With a list of years and months, one can see how many items there are in which period and jump with one operation.
The tree has no role in the timeline, so the same place can be used.

**Rejected alternative 1**
A method that rereads from a specified day by a date input on the toolbar.
Aggregating per-month counts and its API would not be needed, but which periods have photos would not be known until jumping, so we rejected it.

**Rejected alternative 2**
A method that has no means of moving to distant periods and only reads on by scrolling from the newest.
The implementation is the smallest, but reaching old photos would mean reading everything in between, so we rejected it.

**Rejected alternative 3**
A method that leaves the tree on the left and puts years and months in a vertical scrubber at the right edge of the listing.
It is closer to a photo app, but the per-month counts are hard to see and the scrubber would be built in-house, so we rejected it.

**Rejected alternative 4**
A method that leaves the tree on the left and puts the list of years and months in a toolbar popover.
The screen skeleton would not change, but an opening operation would be needed each time to see the years and months, so we rejected it.

#### The Timeline Does Not Reserve the Overall Height and Reads the Continuation

**Decision**
The timeline sets no upper limit on count, does not reserve the overall height in advance, and reads the continuation above and below according to scrolling (§3.10).

**Reason**
If images inside archives are also listed, the count can reach millions.
If the overall height were reserved, one element's height would exceed the browser's limit and the end could no longer be displayed.
Jumping to distant periods is covered by the list of years and months.
The decision that the folder view's listing reserves the overall height ("Reserve the Listing's Overall Height in Advance and Read Only the Pages in the Visible Range" in this section) is not applied to the timeline.

**Rejected alternative 1**
A method that puts an upper limit of around 200,000 items and reserves the overall height as in folder view.
The current listing part could be used as is, but the part beyond the limit would be cut off, and in Libraries with many archives the whole could not be surveyed, so we rejected it.

**Rejected alternative 2**
A method that sets the upper limit at several hundred thousand items, which does not reach the height limit.
The current structure could mostly be used, but with small tiles and on narrow screens the limit would come first, so the count limit would have to be held per screen condition, and we rejected it.

#### Put Daily Headings in the Timeline

**Decision**
The timeline's grid inserts a heading and breaks the row each time the day changes (§3.10).

**Reason**
The presentation is the same as a photo app's, and when the photos are from is understood at a glance.
The cost is that when days with few photos continue, there are many empty rows and headings.

**Rejected alternative 1**
A method that inserts a heading each time the month changes.
Tiles would be packed and the granularity would match the list of years and months, but the day taken would be checked in the info panel or viewer, so we rejected it.

**Rejected alternative 2**
A method that inserts no headings and lines up tiles without a break.
It is the densest, but when the photos are from could be known only from the highlight in the list of years and months, so we rejected it.

#### The Timeline Is Grid Only and Ordered Newest First

**Decision**
The timeline has only the grid, and the order is fixed to newest first (§3.10).

**Reason**
The timeline is a listing for viewing, and old periods can be jumped to from the list of years and months.
The timeline's parts can be kept to one.

**Rejected alternative 1**
A method that allows switching between newest first and oldest first.
One could view from the oldest in order, but the list of years and months and the direction of the headings would also have to be switched to match, so we rejected it.

**Rejected alternative 2**
A method that also allows choosing a list with columns for name, date and time, location, and size.
It would be easy to tell apart by location, but the timeline's parts would become two, the grid and the list, so we rejected it.

#### The Timeline Also Allows Operations Up to Renaming

**Decision**
In the timeline, delete, download, cut, copy, rename, and "Show in folder" can be done.
New folder, paste, upload, and move by drag are not provided (§3.10).

**Reason**
While viewing, one can delete a failed photo, save one locally, put one in the clipboard to move it, and fix a name, without moving to folder view.
The order is decided by timeline date, so even if a name is changed, the timeline date does not change. Among Entries with the same timeline date they are ordered by path, so positions may swap.
Operations that need a destination cannot decide the destination in the timeline, which has no current folder.

**Rejected alternative 1**
A method that only browses in the timeline and does all operations in folder view.
The implementation is the smallest, but to delete an unwanted photo found in the timeline one would have to move to folder view each time, so we rejected it.

**Rejected alternative 2**
A method up to delete, download, cut, and copy, without rename.
It avoids building name editing in the timeline's parts, but one would have to move to folder view merely to fix a name, so we rejected it.

#### The Timeline's Filmstrip Reads the Continuation

**Decision**
The filmstrip of the viewer opened from the timeline lines up the range read and reads the continuation when scrolled to the end (§3.10).

**Reason**
The timeline has no upper limit on count, so the structure that decides the overall width from `totalCount` ("Line Up All the Listing's Entries in the Filmstrip" in this section) cannot be used.
If the same loading as the grid is used, one can move to distant photos with the same operation as folder view.

**Rejected alternative 1**
A method that does not show the filmstrip when opened from the timeline.
The implementation is small, but to move to a distant photo one would have to close the viewer and choose again from the grid, so we rejected it.

**Rejected alternative 2**
A method that lines up only a few images before and after the open image.
Reading the continuation would not be needed, but it becomes the same as the method rejected because there would be no means of moving to distant images in folder view, so we rejected it.

#### Switch Folder View, the Timeline, and Search Mode with One Switch of Three Choices

**Decision**
The toolbar has one switch that lays out "フォルダ" (Folder), "タイムライン" (Timeline), and "検索" (Search) side by side, and it highlights the current one (§3.1).
Neither the toolbar nor the timeline has a separate search button, and the timeline itself has no filtering conditions (§3.6, §3.10).

**Reason**
Search mode spans Libraries like the timeline and has no current folder, so it is a screen of the same rank as the other two, and the switch shows which of the three is open.
If the screen for searching by name is unified into search mode, the timeline's loading and per-month counts need not carry filtering conditions.
Searching from a folder keeps its own entrances, `/` and the context menu, which pass the folder as the start.

**Rejected alternative 1**
A method that keeps a search button next to the two-choice switch.
The entrance would stay where it was, but search mode would be highlighted nowhere and would look like a part of folder view, so we rejected it.

**Rejected alternative 2**
A method that narrows the timeline to Entries whose names match with the words of the search field.
It would be fast because of the index, but the per-month counts would also have to be recounted for each narrowing, so we rejected it.

#### Put the Search Conditions in the Left Area in Place of the Tree

**Decision**
Search mode puts the search terms, Libraries, start, scope, rating, and tags in the left area, where folder view puts the tree (§3.6).

**Reason**
The conditions grew to the Libraries, the start, and several tags, and no longer fit in one toolbar row next to the operations.
In search mode the tree only served to leave the search, since moving by it opened folder view, so the area is better spent on the conditions.
The timeline already uses the left area for its Library checkboxes, so the two screens that span Libraries choose them in the same place.

**Rejected alternative 1**
A method that keeps the conditions in the toolbar row.
No new area would be needed, but a list of tags cannot be laid out there and the row would wrap on medium widths, so we rejected it.

**Rejected alternative 2**
A method that keeps the tree and puts the conditions in a panel on the right.
Moving by the tree would stay, but the panel would compete with the info panel for the same side, so we rejected it.

#### Choose the Start Only While One Library Is Searched, with a Tree in a Popover

**Decision**
The start can be chosen only while one Library is checked, from a tree of that Library's folders and archives opened in a popover. While several Libraries are checked, each Library's whole is searched (§3.6).

**Reason**
A start is a Library-relative path, so it has meaning only within one Library.
A tree lets the user reach a deep folder without knowing its exact path, and shows archives too, so an archive can be chosen as the start to search inside it.

**Rejected alternative 1**
A method that holds a start for each checked Library.
It would allow mixed starts, but the URL and the panel would need a start per Library, and the use of narrowing several Libraries to different folders at once is rare, so we rejected it.

**Rejected alternative 2**
A method that types the start as text such as `Library名/フォルダ` with completions.
No tree would be needed, but the user would have to know or type the path, so we rejected it.

#### Open Search Mode with the Conditions Last Used

**Decision**
The switch's "検索" (Search) opens the conditions last used in search mode while the tab is open, and every Library with no conditions the first time. The `/` key and "このフォルダ内を検索" (Search in this folder) in folder view open that folder as the start with no conditions (§3.6).
The conditions are put in the URL and are not saved in the display preferences.

**Reason**
Going to another mode and coming back to the same search, for example to check an Entry in the timeline, is common, and the switch should return to it as it does for the folder last opened.
The flow of searching under the folder being viewed keeps its own entrances, so it does not need the switch to carry the folder.
Conditions change with each search, so saving them on the server would only make the result of opening `/search` depend on another device's last search.

**Rejected alternative 1**
A method in which the switch always opens the current folder as the start.
It would keep the previous behavior, but the timeline has no current folder, so the switch would act differently by the mode it is pressed from, so we rejected it.

**Rejected alternative 2**
A method in which the switch always opens with no conditions.
It would be predictable, but returning to a search would mean entering the conditions again, so we rejected it.

#### Search Results Allow the Same Operations as the Timeline

**Decision**
In search mode, delete, download, cut, copy, rename, and "Show in folder" can be done, also on a selection that spans Libraries. New folder, paste, upload, and move by drag are not provided (§3.6).

**Reason**
Search results, like the timeline, have no current folder and span Libraries, so the operations of the timeline fit them without change, including splitting a selection into one request per Library.
Operations that need a destination cannot decide it without a current folder.

**Rejected alternative**
A method that does not offer delete, cut, and copy on a selection that spans Libraries.
It would avoid splitting requests, but the timeline already splits them, and refusing them only in search mode would make the same selection behave differently, so we rejected it.

<a id="tree-progressive-disclosure"></a>
#### Show the Tree in Stages and Limit the Child-Presence Check to the Visible Range

**Decision**
The tree updates its display in units of pages, and virtualizes and draws only the visible range and the prefetch margin.
The state of focus, selection, expansion, and scrolling is held by Entry path. When moving off screen by keyboard, the destination is displayed and then focused.
This change is introduced using the existing listing API, and does not make the design of the summary listing or the persistent index a precondition.

Unconfirmed nodes are displayed on the premise that they have children. Expansion by click reads the normal child listing, and is not made a two-stage process of waiting for the child probe and then requesting the listing.
The background probe targets only visible rows, cancels requests that left the screen, and eliminates duplicates. What is checked is whether there are folder-like children (including archives) lined up in the tree.
The dedicated shallow probe stops at the first matching Entry. Limits are set on the scanned count and the elapsed time, and when a limit is hit or it fails, it is left unconfirmed. A finding of none is settled only when the direct children could be checked to the end.
A listing snapshot is not built for the child check, and the all-pages listing API is not substituted. An archive is not extracted merely because of a probe, and if there is no existing index it stays unconfirmed.
Probes are placed at the lowest rank of the background budget. Authorization, the handling of hidden Entries, and invalidation after updates are aligned with listings.

**Reason**
In a tree of about 20,000 folders, about 19 seconds of processing was observed even after communication of all items ([measurement record](../measurements/2026-10-15-browse-performance.md#4-left-tree)).
The change that limits the amount of DOM generation to the displayed range can proceed independently of the listing API's full scan.
If the existing display of treating unconfirmed nodes as expandable is maintained, there is no need to examine the children of all nodes first. Limits on the visible range and on probes prevent the background check from becoming another large scan.

**Rejected alternative**
A method that requests child listings for all fetched folders and settles the expansion arrows first.
The child check alone would add requests and full scans proportional to the number of folders, so we do not adopt it.

<a id="summary-detail-hydration"></a>
#### Show Names First and Fill In Needed Details Later

**Decision**
For name order and kind order of direct children, the summary listing is used, and for size order, modification-date order, and recursive search, the existing detail listing is used. The query keys and snapshots of summary and detail are not mixed.
The grid and list read details in batches from the visible range and prefetch pages, and the tree reads the names needed for display first. Unfinished work that left the screen, and responses of a previous snapshot, are canceled.
Sizes and dates not yet fetched are displayed as not fetched. While metadata is unknown, mutating operations are not permitted, and it is judged by combining the Library's and archive's readonly with the readonly of the Entry that could be fetched.
Selecting by name and opening operations do not wait for details. For the display classification of images and videos, the extension is used as a hint only while details are not fetched, and after fetching, the server's contentType is followed. The archive hint always uses the server's value. Targets used by the info panel and the change menu have their details read with priority, and after fetching, the selection's attributes are also updated by path. If any are unfetched in the total, it is not displayed as a complete total size.
Adding details does not change the order by name and kind. When the kind or archive hint changes, the target listing is updated.
An authenticated capabilities query that does not depend on the target path is read first, and only when the feature is not provided does it fall back to the detail listing. A 404/405/501 of the capabilities query itself is treated as an old daemon, but errors of the listing and details after it answered that the feature is provided are not hidden by the fallback. Authentication, permission, and ordinary I/O errors are also not treated as compatibility issues.

**Reason**
It separates the first display from stat on all items while avoiding the wrong display of placeholder sizes or writable.

<a id="visible-child-probes"></a>
#### Check Whether There Are Children Shallowly, Only for Visible Rows

**Decision**
An unconfirmed folder-like Entry is displayed as expandable. Click reads the normal child listing immediately and does not wait for the probe to complete.
What is checked in the background is only visible rows, and requests that left the visible range are discarded if not yet sent and canceled if sent. The web client sends probes one request at a time, and uses backoff with an upper limit of 0.5 to 30 seconds for unconfirmed responses.
If there is an existing fresh summary index, the probe uses the count of folder-like children; otherwise, it uses a dedicated shallow enumeration that stops at the point it finds the first folder-like child. Archive hints are also counted as children, but it does not descend through symlinks.
It does not substitute reading all pages of the normal listing API. The handling of hidden Entries and authentication and boundaries are aligned with the normal listing.

A probe after authentication and path syntax validation is limited to at most 128 Entries and a response wait of 50 ms, including boundary checking and worker waiting, and returns unconfirmed if a slot cannot be obtained, the count or time is exceeded, or it fails midway. Empty is settled only when the target's direct children could be checked to EOF.
A blocking syscall that exceeded the time is not treated as forcibly stopped; the shared slot is held until it actually returns, and it does not move on to the next Entry.
An archive is checked only when an already opened index can be used, and expansion or building a new heavy index is not started merely for a probe.
Unknown versions and local updates within the last 2 seconds return unconfirmed and the result is not retained. When reflecting our own changes, the server's probe results are also invalidated. Retention is at most 4096 targets and 30 seconds, and is discarded if the version changes.
Check results are tied to the source's version, and the web client discards them if the parent listing's snapshot changes. The server attaches the remaining validity time maxAgeMs to the response, and the web client does not retain results beyond it. The validity is measured from receiving the response, and a retained result resent does not get a new 30 seconds. After expiry, visible rows are checked again. The maxAgeMs of unconfirmed is 0.
If capabilities' childProbes is false, or the capabilities query is not provided in an old daemon, it does not probe, stays unconfirmed, and uses the normal child listing.

**Reason**
Unconfirmed is not misjudged as empty, and folder checks of all 20,000 items do not become new background load.

#### Add Double Tap and a Downward Swipe to the Viewer's Touch Operations

**Decision**
In addition to left and right flicks and pinch, the viewer accepts a double tap to switch zoom and a downward swipe to close (§3.4).
Flicks and the swipe are accepted only while the image is shown whole, and not in the video viewer.
The gestures are recognized with the Pointer Events the viewer already uses for dragging, without adding a gesture library.

**Reason**
A narrow screen fetches the original image only on zoom by default, and a double tap reaches the enlarged view with one hand.
A downward swipe closes the viewer without reaching for the button at the top of a phone screen.
Taking the gestures in the image area keeps them from starting the browser's pull-to-refresh, which a downward swipe would otherwise collide with.
Limiting flicks and the swipe to the whole view keeps them from competing with dragging a zoomed image, and keeping them off videos keeps them from competing with the player's seek bar.
The set of gestures is small, so recognizing them in the existing pointer handling does not need a dependency.

**Rejected alternative 1**
A method that implements only left and right flicks and pinch.
There would be fewer gestures to tell apart, but reaching the enlarged view and closing would need two fingers or the button at the top, so we rejected it.

**Rejected alternative 2**
A method that also moves back and forth among videos with flicks.
Movement would be the same for images and videos, but a horizontal drag on the seek bar would be taken as a flick, so we rejected it.

### 5.2 Deferred

#### Whether to Show Counts on Folder Tiles

**Current state**
The tiles of folders and archives show only the representative thumbnail and the name.
The listing response does not have the number of Entries inside a folder.

**Why not decide now**
To show counts, the contents of each folder in the level being listed would have to be counted, and the cost of displaying one level would increase by the number of folders.
Whether counts are needed for viewing cannot be known until the screen has been used.

**Conditions for deciding**
When, after actually using it, situations arise repeatedly in which one wants to know the amount of contents before opening a folder.

#### Whether to Download Multiple Entries at Once

**Current state**
The web client's download targets only one file and fetches from the content API (§3.7).
There is no API that fetches multiple files and folders together.

**Why not decide now**
To fetch together, we would need to decide an API that bundles the selected Entries into a ZIP or similar and returns it as a stream, whether to compress, and how to handle Entries inside archives.
We do not know how much demand there is for taking things out together to local.

**Conditions for deciding**
When situations of taking multiple files or folders out to local arise repeatedly in actual use.

#### Whether to Filter Listings by Kind

**Current state**
Listings and search can be narrowed by name, but not by kinds such as images only or videos only.
The previous search API (`searchEntries`) could narrow by an Entry's `kind` (file or folder), but the web client did not use it.

**Why not decide now**
To narrow by kind, a condition would have to be added to the listing API and its application to snapshots decided ([Listing and Search Design §4.1](./listing.md#41-the-unit-that-is-built)).
We do not know whether there is demand to view images and videos separately in a folder where they are mixed.

**Conditions for deciding**
When situations arise repeatedly in which one wants to view only one of images or videos in a folder where they are mixed.

#### Whether to Show an Entry's Details from the Viewer

**Current state**
The info panel is placed to the right of the listing (§3.8) and is not visible while the viewer is open.
The viewer has no button that shows an Entry's details.

**Why not decide now**
If details were shown in the viewer, the same contents would be shown under the same request rules as the info panel (§3.8).
Until the info panel is used on real screens, we cannot judge whether overlaying that part on the viewer is enough.

**Conditions for deciding**
When the info panel has been used on real screens and one can judge whether overlaying the same part on the viewer is enough.

## 6. Current State and Remaining Work

### 6.1 Current Code

Summary listings and detail completion are implemented in [entries.ts](../../ui-web/src/library/entries.ts) and [listing.ts](../../ui-web/src/library/listing.ts). Support is confirmed with capabilities, and names and kinds are displayed first. The existence of children for visible rows is checked one request at a time by [ChildProbeQueue](../../ui-web/src/library/childProbes.ts). It cancels on leaving the screen and on snapshot change, and returns to unconfirmed after the remaining TTL. Waiting for a probe does not stop normal expansion.

The web client is a Vite SPA that the daemon serves from `[web] dir`.
It has initial setup and login, the toolbar (mode switch, breadcrumb, user menu), the tree, the grid and list, the status bar, selection, the image channel client, flipbook playback of videos, and the image viewer.
The video viewer chooses direct playback or HLS from media info, and on closing ends playback and the transcode session.
The screen skeleton lines up the toolbar, tree, listing, and status bar on a wide screen (the tree and info panel change width with [ResizeHandle.tsx](../../ui-web/src/components/ResizeHandle.tsx)), and folds the tree into a drawer on a narrow screen. The color scheme follows the OS's light and dark setting.
Display preferences wait for loading at the entrance of authenticated screens, and run with defaults if they cannot be read. Changes are written back as a whole when they have stopped for 500 ms, and on failure return to the saved values.
From the toolbar's settings menu ([SettingsMenu.tsx](../../ui-web/src/components/SettingsMenu.tsx)), the listing display, the grid's tile size, the thumbnail shape, and sorting can be chosen for the set currently in use. Wide and narrow screens are separate sets, and they switch when the screen width crosses 768 CSS px. The list's set does not show the tile size choice.
The listing's data is held per page by [library/listing.ts](../../ui-web/src/library/listing.ts). It reserves the overall height from the first page's `totalCount`, and reads the pages containing the visible rows and one screen of prefetch rows in the scrolling direction, 200 items at a time by `offset`. It shows frames at unread positions and cancels requests for pages that left the targets.
The grid lines up square tiles following the tile size level. The list, on a wide screen, has the 5 columns of small thumbnail, name, modification date, size, and kind with sort headings, and on a narrow screen shows no headings and lines up two-line rows of the name and "更新日時 · サイズ" (modification date · size). On a narrow screen, a single tap on a tile or list row opens it ([useEntryInteraction.ts](../../ui-web/src/library/useEntryInteraction.ts)).
When the folder, sorting, listing display, or search conditions change, the display returns to starting from the top. When a page with a different snapshot id arrives, the other pages are discarded and the needed range is reread, keeping the scroll position.
Range selection reads unread pages first, shows progress in the status bar, and selects after reading finishes. Loading is canceled by another operation, a folder move, or a snapshot change. It holds the selected paths and the size, kind, and changeability at the time of selection, so the count and total size, the destination of operations, and the selectable state can be decided regardless of the pages being loaded. On sorting and snapshot change only the start is discarded, and on changes of the folder, listing display, or search conditions the selection is cleared.
Search is handled by search mode in [SearchPage.tsx](../../ui-web/src/pages/SearchPage.tsx), which spans Libraries like the timeline: its Entries carry `/{libraryId}{Library-relative path}` as their path, and it shares the grid, list, viewer, selection, info panel, and Entry operations with the timeline's rules (§3.6). The conditions are in the left area ([SearchPanel.tsx](../../ui-web/src/components/SearchPanel.tsx)) in place of the tree: the search terms field, a checkbox per Library, the start (a popover holding the picking mode of [Tree.tsx](../../ui-web/src/components/Tree.tsx), only while one Library is checked), "サブフォルダを含めない", the rating, and a checkbox per tag word. Terms are put in the URL when input stops for 300 ms ([useDebouncedParam.ts](../../ui-web/src/library/useDebouncedParam.ts)), and the other conditions immediately, replacing the URL. On a narrow screen only the terms field stays under the header and the rest is in the drawer, whose button shows the number of narrowing conditions. The last conditions are kept for the tab by [searchReturn.ts](../../ui-web/src/library/searchReturn.ts) for the switch's "検索". The old `/search/{libraryId}` and the `/browse` with `q` and `deep=1` are replaced with a `/search` URL.
The paged listing store in [listing.ts](../../ui-web/src/library/listing.ts) reads either a folder or a search (`ListingSource`), and a search is requested with `listSearchEntries` through `fetchSearchPage` in [entries.ts](../../ui-web/src/library/entries.ts), which turns each Entry's path into the keyed path. Grid, List, and the viewer take the source; the viewer moves among the search results. For subtree results, the parent folder is shown below the name in the grid, in the "場所" (Location) column in a wide screen's list, and at the start of the second line in a narrow screen's list; direct children of the start are shown as ".". During a subtree search, "検索中…" (Searching…) is displayed, and if the search results are 0, "一致する項目がありません。" (No matching items.) is displayed, and on truncation of any Library it guides narrowing the Libraries or moving the start down and searching again. Opening a folder or archive result moves to that position in folder view and ends the search.
The tree reads folder-like Entries in order by `offset` and shows children after fetching all pages. When the snapshot's id changes, it rereads from the start.
The image and video viewer also uses the same listing data, and reads pages in order until the Entry of the relative path that `view` of a directly opened URL points to is found. Moving to the previous and next Entries stops at images and videos and skips the rest, and moves in one operation after finishing reading unread pages too. When the destination is known, it is opened at that position without waiting for the search. The display images of the image before and after in the loaded range are prefetched. The original image is fetched at a timing decided from the format, size, and the display preferences' value for swapping to the original image, and the size of the display image request is also decided by the same result ([viewer.ts](../../ui-web/src/viewer/viewer.ts)). Zoom is done only with the wheel.
The wide-screen image and video viewer has the filmstrip at the bottom edge ([Filmstrip.tsx](../../ui-web/src/components/Filmstrip.tsx)), whose item side follows the filmstrip size, and the image viewer has the header's slideshow, full-screen, download, and delete buttons. The header's gear opens the viewer settings panel ([ViewerSettings.tsx](../../ui-web/src/components/ViewerSettings.tsx)) with the filmstrip size and the swapping to the original image. Rate mode's buttons and undo history are in [Viewer.tsx](../../ui-web/src/components/Viewer.tsx). On narrow screens, the slideshow and full-screen are placed in the "その他" (More) menu, the gear of an image has only the swapping to the original image, and the filmstrip, download, and delete are not shown. Confirmation dialogs, menus, and toasts are drawn inside the outer frame element of `Modal`, and full-screen display is also performed on that element (§5.1).
Parts are built with Tailwind, the shadcn/ui parts in `ui-web/src/components/ui`, and in-house code.
On a wide screen, from the listing's context menu and the toolbar, one can create folders, rename, delete, and download a single file. Creating, renaming, and deleting can also use key operations. In the tree too, one can create inside, rename, and delete the right-clicked folder. Creating and renaming are done in the modal dialog of [NameDialog.tsx](../../ui-web/src/components/NameDialog.tsx), which shows the target's path. Whether something can be changed is decided by the logical OR of the Library's `readonly` and the Entry's or the current folder's `metadata.readonly`. After success, that Library's listing and tree are reread.
[clipboard.ts](../../ui-web/src/library/clipboard.ts) holds the targets of cut and copy across authenticated screens. They can be operated with the menu and Ctrl or ⌘ X, C, and V, and during cut the targets in the grid and list are shown faintly. Paste targets the current folder, or a tile or tree folder, and transfer to another Library is also possible.
[TransferProvider.tsx](../../ui-web/src/components/TransferProvider.tsx) checks the destination's writability and Entries of the same name, and if there are conflicts, asks the policy only once. It splits the requests for files and folders, notifies synchronous success and rereads the listing and tree, and keeps accepted Operations even if there are partial failures.
Internal drag moves to the same Library's tree and folder tiles, and copies when Alt is pressed. It rejects destinations that are readonly or archives, scrolls at the listing's edges, and expands closed tree folders after 1 second.
[upload.ts](../../ui-web/src/library/upload.ts) sends the toolbar's file and folder selection and OS drops in a queue of at most 2. Files of 32 MiB or less are written directly, and larger files send 16 MiB chunks. Folders are created first, and the conflict policy is also applied to what is underneath. For OS folders, it reads from the same reader until an empty batch, and does not drop the tail of what is underneath ([Measurements of Chromium](../knowledge/chromium-directory-reader.md)). A session is discarded when canceled, and the listing and tree are reread once per published file.
The activity panel is shown overlaid at the bottom right and shows the progress, state, and failures and skips of Operations and uploads. Operations are updated every second only while running, and return even after a reload. Uploads show the number of bytes sent and can be canceled while waiting and sending. When collapsed it becomes a status bar button and shows the unread count. The items lined up when it is opened become read and are shown faintly, and finished items can be removed with ×. During an upload a leave confirmation is registered, and it is released when a terminal state is reached. On narrow screens, the entrances to operations and the panel are not shown.
The viewer does not have back-and-forth movement by left and right flicks, zoom by pinch or double tap, or closing by a downward swipe.
Thumbnails, sprite sheets, and display images are received only over the image channel (IC2): one client per tab ([channel.ts](../../ui-web/src/images/channel.ts)) merges the wants of the grid, list, timeline, info panel, filmstrip, and viewer into items with a class and an order, and keeps received images in IndexedDB with the budgets of §4 ([cache.ts](../../ui-web/src/images/cache.ts)). A part subscribes with [useImageWants](../../ui-web/src/images/wants.ts) or [DisplayImages](../../ui-web/src/viewer/displayImages.ts) and gets a blob URL. A cache hit is shown at once and wanted again with its `ETag`, so an unchanged file costs no image transfer. The per-part `ThumbnailQueue`, `fetchThumbnail`, and `loadDisplayImage`, and the browser's HTTP cache of thumbnails, are gone. Logging out clears the cache.
The image channel client does not yet discard the states of folder and archive tiles after the daemon's own changes, and search mode does not display the notice for results the name index did not answer.
To the right of the listing on a wide screen, the info panel ([InfoPanel.tsx](../../ui-web/src/components/InfoPanel.tsx)), which can be opened and closed with a toolbar button, is placed.
The open/closed state is saved in the browser's `localStorage`, and is open when first opened.
When one item is selected, the name, kind, size, modification date, and Library-relative path are shown from the Entry the selection holds, without a request.
Size is not shown for folders and archives.
When multiple items are selected, the count and total size are shown.
The larger thumbnail and media info are requested only when the selection has not changed for 200 ms, and when the selection changes, the request already sent is canceled.
For images, it shows the dimensions with the rotation applied and the items of shooting info that Exif has, and for videos the duration, resolution, and codecs.
The timeline (§3.10) places at `/timeline` a grid with daily headings ([TimelineGrid.tsx](../../ui-web/src/components/TimelineGrid.tsx)) and a list of years and months ([TimelineSidebar.tsx](../../ui-web/src/components/TimelineSidebar.tsx)).
Loading is held by `TimelineStore` in [library/timeline.ts](../../ui-web/src/library/timeline.ts), which points to Entries by absolute positions based on the start of the range first read, so even when reading is added toward the front, the positions of the selection and the viewer do not move.
The viewer and filmstrip run on a `TimelineStore` that has the same `Listing` shape, and use the same parts as the folder view listing.
The switch between folder view and the timeline is [ModeSwitch.tsx](../../ui-web/src/components/ModeSwitch.tsx), which goes back to the folder saved by [folderReturn.ts](../../ui-web/src/library/folderReturn.ts) in `localStorage`, and "Show in folder" is received by `reveal` in the folder view URL.
Timeline Entries carry `/{libraryId}{Library-relative path}` as their path, so the selection, the viewer, and the positions in the range tell Entries of different Libraries apart without change; the API helpers turn that path back into the Library and the Library-relative path with `entryAddress` in [location.ts](../../ui-web/src/location.ts).
Ratings and tags are read and written by the queries and saving in [ratingTags.ts](../../ui-web/src/library/ratingTags.ts). In the info panel, [EntryTags.tsx](../../ui-web/src/components/EntryTags.tsx) handles displaying the rating and attaching and detaching tags. The viewer's rate mode is handled by [Viewer.tsx](../../ui-web/src/components/Viewer.tsx), shared by the viewers of folder view, search mode, and the timeline.
The tree subscribes with [useTreeListing.ts](../../ui-web/src/library/useTreeListing.ts) to levels that are expanded and whose ancestors are not closed, and shares the page and generation management of [listing.ts](../../ui-web/src/library/listing.ts).
It generates only the rows in the visible range, and makes unfetched positions frames that have height. For key operations toward off-screen, it reads the destination page and then focuses.
The dedicated background probe for checking the presence of children is not yet implemented. It is follow-up work that follows the rules in §5.1 for the visible range, shallow probing, and load limits.

### 6.2 Roadmap

The order is provisional.
Only the dependencies must be respected.

| No. | Content | Dependencies assumed |
| --- | --- | --- |
| C10 | Implement back-and-forth movement by left and right flicks in the viewer, zoom by pinch and double tap, and closing by a downward swipe | None |
| C11 | Discard the states of folder and archive tiles and want them again when the listing is reread after the daemon's own change (§4) | None |
| C12 | Display the notice of §3.6 when a subtree search from a folder outside archives is not answered from the name index | L1 of [Listing and Search Design §9.2](./listing.md#92-roadmap) |

C10 corresponds to M15, C11 to part of M20, and C12 to part of M19 of [design.md §12.2](../design.md#122-roadmap).
M13, which corresponds to C8b, M14, which corresponds to C9a and C9b, and M17, which corresponds to the timeline screen, are complete.
With the narrow-screen default of "only on zoom" as is, the state in which the original image cannot be shown by touch operation alone continues until C10.

For confirmed defects, see [issues.md](../issues.md).
