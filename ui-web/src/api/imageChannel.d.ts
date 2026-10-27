/* Generated from daemon/image-channel.schema.json by npm run gen:channel. Do not edit by hand. */

/**
 * Everything that crosses the image channel (GET /api/v1/image-channel, upgraded to a WebSocket). The client sends the text messages of ClientMessage. The server sends the text messages of ServerMessage and the binary frames of an image: a 4-byte big-endian length, a FrameHeader as UTF-8 JSON of that length, and one chunk of the image. This is the contract; docs/design/image-channel.md gives the reasons.
 */
export type ImageChannelContract = ClientMessage | ServerMessage | FrameHeader;
/**
 * A text message from the client, at most 256 KiB.
 */
export type ClientMessage = Want | Drop;
/**
 * Names an item within one connection. The client chooses it, and every message about the item carries it.
 */
export type ItemId = number;
/**
 * Which image of the entry an item wants. A sprite sheet is for a video only.
 */
export type ItemKind = "thumbnail" | "sprite" | "display";
/**
 * How a thumbnail or a sprite sheet fits its square: crop fills it, fit keeps the aspect ratio inside it. A display image always fits.
 */
export type Shape = "crop" | "fit";
/**
 * How urgently an item is wanted: shown now in the viewer, on screen, or expected soon.
 */
export type RequestClass = "interactive" | "visible" | "prefetch";
/**
 * A text message from the server.
 */
export type ServerMessage = Unchanged | Failed;
/**
 * The kind of the API's error response (openapi.yaml, ErrorKind).
 */
export type ErrorKind =
  | "InternalServerError"
  | "InvalidRequest"
  | "UnsupportedType"
  | "LimitExceeded"
  | "NotFound"
  | "Unauthorized"
  | "Forbidden"
  | "PasswordRequired"
  | "AlreadyInitialized"
  | "RangeNotSatisfiable"
  | "Conflict"
  | "PayloadTooLarge"
  | "InsufficientStorage"
  | "Timeout";

/**
 * Adds items to the wanted set. An id that is already in the set may only change its class, order, and etag; changing what it asks for closes the channel with 4400.
 */
export interface Want {
  type: "want";
  /**
   * @maxItems 4096
   */
  items: WantedItem[];
}
/**
 * One image the client wants. The output format, resampler, and anchor are the server's configuration, so an item does not carry them. Width and height round up to the server's size steps; a thumbnail or a sprite sheet without them is 256, and a display image without them fits the largest display size.
 */
export interface WantedItem {
  id: ItemId;
  libraryId: string;
  /**
   * The Library-relative path of a file, a folder, or an archive.
   */
  path: string;
  kind: ItemKind;
  width?: number;
  height?: number;
  shape?: Shape;
  class: RequestClass;
  /**
   * The place of the item within its class, smaller first.
   */
  order: number;
  /**
   * The ETag of the image the client holds for this item. If it is still the current one, the server answers unchanged and sends no image.
   */
  etag?: string;
}
/**
 * Removes items from the wanted set. An id that is not in the set is ignored.
 */
export interface Drop {
  type: "drop";
  /**
   * @maxItems 4096
   */
  ids: ItemId[];
}
/**
 * The image the client holds for the item is still the current one. The item leaves the wanted set.
 */
export interface Unchanged {
  type: "unchanged";
  id: ItemId;
}
/**
 * The image of the item could not be made. The item leaves the wanted set.
 */
export interface Failed {
  type: "failed";
  id: ItemId;
  kind: ErrorKind;
  message?: string;
}
/**
 * The header of a binary frame, which carries the chunk of an image from offset. The image is complete when the chunks cover total bytes. Frames for an item that the client has dropped are ignored.
 */
export interface FrameHeader {
  id: ItemId;
  /**
   * The ETag of the image, or null for an image that has no cache key and cannot be revalidated.
   */
  etag: string | null;
  contentType: "image/webp" | "image/jpeg" | "image/png";
  /**
   * How many frames a sprite sheet holds, and null for any other image.
   */
  frameCount: number | null;
  offset: number;
  total: number;
}
