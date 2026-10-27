//! The messages of the image channel. `daemon/image-channel.schema.json` is the contract, and the tests check these types against it.
use serde::{Deserialize, Serialize};

use omnius_pxna_interface::models;

use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemKind {
    Thumbnail,
    Sprite,
    Display,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Shape {
    Crop,
    Fit,
}

/// How urgently an item is wanted. The order of the variants is the order they are served in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RequestClass {
    Interactive,
    Visible,
    Prefetch,
}

/// One image the client wants.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WantedItem {
    pub id: u64,
    pub library_id: String,
    pub path: String,
    pub kind: ItemKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<Shape>,
    pub class: RequestClass,
    pub order: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
}

impl WantedItem {
    /// The largest id a JavaScript number holds exactly.
    const MAX_ID: u64 = 9_007_199_254_740_991;
    const MAX_SIDE: u32 = 4096;

    /// Checks what the schema checks and a deserializer cannot: the ranges. An item that fails is a break of the contract.
    pub fn validate(&self) -> Result<()> {
        let invalid = |message: &str| Err(Error::new(ErrorKind::InvalidInput).with_message(format!("item {}: {message}", self.id)));
        if self.id > Self::MAX_ID {
            return invalid("id is out of range");
        }
        if self.library_id.is_empty() || self.path.is_empty() {
            return invalid("libraryId and path must not be empty");
        }
        if [self.width, self.height].into_iter().flatten().any(|side| !(1..=Self::MAX_SIDE).contains(&side)) {
            return invalid("width and height must be from 1 to 4096");
        }
        if !self.order.is_finite() || self.order < 0.0 {
            return invalid("order must not be negative");
        }
        if self
            .etag
            .as_ref()
            .is_some_and(|etag| etag.len() != 64 || !etag.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte)))
        {
            return invalid("etag must be 64 lowercase hex digits");
        }
        Ok(())
    }

    /// Whether `other` asks for the same image. A re-sent item may change its class, order, and etag, and nothing else.
    pub fn asks_for_the_same_image(&self, other: &Self) -> bool {
        (&self.library_id, &self.path, self.kind, self.width, self.height, self.shape) == (&other.library_id, &other.path, other.kind, other.width, other.height, other.shape)
    }
}

/// A text message from the client.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ClientMessage {
    Want { items: Vec<WantedItem> },
    Drop { ids: Vec<u64> },
}

/// A text message from the server.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ServerMessage {
    Unchanged {
        id: u64,
    },
    Failed {
        id: u64,
        kind: models::ErrorKind,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    },
}

/// The header of a binary frame: where in the image its chunk goes.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrameHeader {
    pub id: u64,
    pub etag: Option<String>,
    pub content_type: String,
    pub frame_count: Option<u32>,
    pub offset: u64,
    pub total: u64,
}

/// Writes and reads the binary frame: a 4-byte big-endian length, the header as JSON of that length, and the chunk.
pub struct FrameCodec;

impl FrameCodec {
    pub fn encode(header: &FrameHeader, chunk: &[u8]) -> Result<Vec<u8>> {
        let header = serde_json::to_vec(header).map_err(|error| Error::from_error(error, ErrorKind::UnexpectedError))?;
        let length = u32::try_from(header.len())?;
        let mut frame = Vec::with_capacity(4 + header.len() + chunk.len());
        frame.extend_from_slice(&length.to_be_bytes());
        frame.extend_from_slice(&header);
        frame.extend_from_slice(chunk);
        Ok(frame)
    }

    #[cfg(test)]
    pub fn decode(frame: &[u8]) -> Result<(FrameHeader, &[u8])> {
        let invalid = || Error::new(ErrorKind::InvalidFormat).with_message("a frame is shorter than its header says");
        let length = u32::from_be_bytes(frame.get(..4).ok_or_else(invalid)?.try_into()?) as usize;
        let header = frame.get(4..4 + length).ok_or_else(invalid)?;
        let header = serde_json::from_slice(header).map_err(|error| Error::from_error(error, ErrorKind::InvalidFormat))?;
        Ok((header, &frame[4 + length..]))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use testresult::TestResult;

    use super::*;

    /// Validates `instance` against one definition of the contract.
    fn validate(definition: &str, instance: &Value) -> std::result::Result<(), String> {
        let mut schema: Value = serde_json::from_str(include_str!("../../../../../../image-channel.schema.json")).expect("the schema is JSON");
        schema.as_object_mut().expect("the schema is an object").remove("anyOf");
        schema["$ref"] = json!(format!("#/$defs/{definition}"));
        let validator = jsonschema::validator_for(&schema).expect("the schema is a schema");
        validator.validate(instance).map_err(|error| error.to_string())
    }

    fn item() -> WantedItem {
        WantedItem {
            id: 7,
            library_id: "photos".to_string(),
            path: "/a/b.heic".to_string(),
            kind: ItemKind::Thumbnail,
            width: Some(256),
            height: Some(256),
            shape: Some(Shape::Fit),
            class: RequestClass::Visible,
            order: 2.5,
            etag: Some("a".repeat(64)),
        }
    }

    #[test]
    fn the_messages_the_daemon_serializes_match_the_schema_test() -> TestResult {
        let want = ClientMessage::Want { items: vec![item()] };
        let minimal = ClientMessage::Want {
            items: vec![WantedItem {
                width: None,
                height: None,
                shape: None,
                etag: None,
                kind: ItemKind::Display,
                class: RequestClass::Interactive,
                ..item()
            }],
        };
        let drop = ClientMessage::Drop { ids: vec![1, 2, 3] };
        for message in [want, minimal, drop] {
            let value = serde_json::to_value(&message)?;
            validate("ClientMessage", &value)?;
            assert_eq!(
                serde_json::from_value::<ClientMessage>(value)?,
                message,
                "the schema and the types agree on what a message is"
            );
        }

        let unchanged = ServerMessage::Unchanged { id: 4 };
        let failed = ServerMessage::Failed {
            id: 5,
            kind: models::ErrorKind::NotFound,
            message: Some("no image".to_string()),
        };
        let failed_without_message = ServerMessage::Failed {
            id: 6,
            kind: models::ErrorKind::Timeout,
            message: None,
        };
        for message in [unchanged, failed, failed_without_message] {
            let value = serde_json::to_value(&message)?;
            validate("ServerMessage", &value)?;
            assert_eq!(serde_json::from_value::<ServerMessage>(value)?, message);
        }

        for header in [
            FrameHeader {
                id: 1,
                etag: Some("0".repeat(64)),
                content_type: "image/webp".to_string(),
                frame_count: Some(8),
                offset: 65536,
                total: 100_000,
            },
            FrameHeader {
                id: 2,
                etag: None,
                content_type: "image/jpeg".to_string(),
                frame_count: None,
                offset: 0,
                total: 0,
            },
        ] {
            validate("FrameHeader", &serde_json::to_value(&header)?)?;
        }
        Ok(())
    }

    #[test]
    fn the_schema_and_the_types_refuse_the_same_messages_test() {
        let bad_messages = [
            json!({ "type": "want" }),
            json!({ "type": "want", "items": [{ "id": 1, "libraryId": "a", "path": "/x", "kind": "thumbnail", "class": "visible" }] }),
            json!({ "type": "want", "items": [{ "id": 1, "libraryId": "a", "path": "/x", "kind": "poster", "class": "visible", "order": 0 }] }),
            json!({ "type": "want", "items": [{ "id": 1, "libraryId": "a", "path": "/x", "kind": "thumbnail", "class": "urgent", "order": 0 }] }),
            json!({ "type": "want", "items": [{ "id": 1, "libraryId": "a", "path": "/x", "kind": "thumbnail", "class": "visible", "order": -1 }] }),
            json!({ "type": "want", "items": [{ "id": 1, "libraryId": "a", "path": "/x", "kind": "thumbnail", "class": "visible", "order": 0, "width": 0 }] }),
            json!({ "type": "want", "items": [{ "id": 1, "libraryId": "a", "path": "/x", "kind": "thumbnail", "class": "visible", "order": 0, "width": 4097 }] }),
            json!({ "type": "want", "items": [{ "id": 1, "libraryId": "a", "path": "/x", "kind": "thumbnail", "class": "visible", "order": 0, "etag": "ABC" }] }),
            json!({ "type": "want", "items": [{ "id": 1, "libraryId": "a", "path": "/x", "kind": "thumbnail", "class": "visible", "order": 0, "format": "jpeg" }] }),
            json!({ "type": "drop", "ids": ["1"] }),
            json!({ "type": "drop" }),
            json!({ "type": "reload" }),
        ];
        for message in bad_messages {
            assert!(validate("ClientMessage", &message).is_err(), "the schema accepted {message}");
            let refused = match serde_json::from_value::<ClientMessage>(message.clone()) {
                Err(_) => true,
                Ok(ClientMessage::Want { items }) => items.iter().any(|item| item.validate().is_err()),
                Ok(ClientMessage::Drop { .. }) => false,
            };
            assert!(refused, "the types accepted {message}");
        }
    }

    #[test]
    fn an_item_may_change_its_class_order_and_etag_only_test() {
        let item = item();
        let moved = WantedItem {
            class: RequestClass::Prefetch,
            order: 9.0,
            etag: None,
            ..item.clone()
        };
        assert!(item.asks_for_the_same_image(&moved));
        for changed in [
            WantedItem {
                path: "/other".to_string(),
                ..item.clone()
            },
            WantedItem { width: Some(512), ..item.clone() },
            WantedItem {
                kind: ItemKind::Display,
                ..item.clone()
            },
            WantedItem {
                shape: Some(Shape::Crop),
                ..item.clone()
            },
        ] {
            assert!(!item.asks_for_the_same_image(&changed));
        }
    }

    #[test]
    fn a_frame_round_trips_through_its_length_and_header_test() -> TestResult {
        let header = FrameHeader {
            id: 3,
            etag: None,
            content_type: "image/png".to_string(),
            frame_count: None,
            offset: 10,
            total: 14,
        };
        let frame = FrameCodec::encode(&header, b"data")?;
        let (decoded, chunk) = FrameCodec::decode(&frame)?;
        assert_eq!(decoded, header);
        assert_eq!(chunk, b"data");
        assert!(FrameCodec::decode(&frame[..3]).is_err());
        assert!(FrameCodec::decode(&frame[..6]).is_err());
        Ok(())
    }
}
