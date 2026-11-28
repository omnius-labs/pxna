//! The image socket: the WebSocket through which the web client wants thumbnails, sprite sheets, and display images
//! (Image Socket Design). `daemon/sockets/image.schema.json` is its contract, and `openapi.yaml` lists only the path of the upgrade.
mod connection;
mod message;
mod route;
#[cfg(test)]
mod tests;

pub use route::*;
