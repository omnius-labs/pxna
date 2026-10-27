//! What every image channel shares: the limits, how many channels each session has open, and the signal that closes them (Image Channel Design §3).
use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio_util::sync::CancellationToken;

use crate::prelude::*;

/// The limits of one image channel and of one session's channels.
#[derive(Debug, Clone)]
pub struct ImageChannelLimits {
    /// How many items one connection may want at the same time.
    pub max_items: usize,
    /// The size of one text message.
    pub max_text_bytes: usize,
    /// How many channels one session may have open.
    pub max_channels_per_session: usize,
    /// How many items one connection has in generation at the same time.
    pub max_generating: usize,
    /// How many items one connection checks against an `ETag` at the same time. These need no image, so they do not count as generating.
    pub max_checking: usize,
    /// How many bytes of finished images may wait to be sent before the connection passes no more items to generation.
    pub max_unsent_bytes: usize,
    /// The largest piece of an image in one binary frame.
    pub chunk_bytes: usize,
    /// How often both sides ping. The session is also checked at each ping.
    pub ping_interval: Duration,
}

impl Default for ImageChannelLimits {
    fn default() -> Self {
        Self {
            max_items: 4096,
            max_text_bytes: 256 * 1024,
            max_channels_per_session: 8,
            max_generating: 8,
            max_checking: 16,
            max_unsent_bytes: 4 * 1024 * 1024,
            chunk_bytes: 64 * 1024,
            ping_interval: Duration::from_secs(30),
        }
    }
}

pub struct ImageChannels {
    limits: ImageChannelLimits,
    open: Mutex<HashMap<String, usize>>,
    closing: CancellationToken,
}

impl ImageChannels {
    pub fn new(limits: ImageChannelLimits) -> Arc<Self> {
        Arc::new(Self {
            limits,
            open: Mutex::new(HashMap::new()),
            closing: CancellationToken::new(),
        })
    }

    pub fn limits(&self) -> &ImageChannelLimits {
        &self.limits
    }

    /// Cancelled when the daemon shuts down, so that every channel closes with 1001 and the server can stop.
    pub fn closing(&self) -> &CancellationToken {
        &self.closing
    }

    /// Counts a channel of the session. `None` when the session already has the most it may have open.
    pub fn open(self: &Arc<Self>, session_id: &str) -> Option<ChannelSlot> {
        let mut open = self.open.lock();
        let count = open.entry(session_id.to_string()).or_default();
        if *count >= self.limits.max_channels_per_session {
            return None;
        }
        *count += 1;
        Some(ChannelSlot {
            channels: self.clone(),
            session_id: session_id.to_string(),
        })
    }

    /// How many channels the session has open. The tests use it to check that closed channels are not counted.
    #[cfg(test)]
    pub fn open_count(&self, session_id: &str) -> usize {
        self.open.lock().get(session_id).copied().unwrap_or(0)
    }
}

/// A channel of a session that is counted as open until this is dropped.
pub struct ChannelSlot {
    channels: Arc<ImageChannels>,
    session_id: String,
}

impl Drop for ChannelSlot {
    fn drop(&mut self) {
        let mut open = self.channels.open.lock();
        if let Some(count) = open.get_mut(&self.session_id) {
            *count -= 1;
            if *count == 0 {
                open.remove(&self.session_id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_session_may_have_a_limited_number_of_open_channels_test() {
        let channels = ImageChannels::new(ImageChannelLimits {
            max_channels_per_session: 2,
            ..ImageChannelLimits::default()
        });
        let first = channels.open("a").expect("the first channel");
        let _second = channels.open("a").expect("the second channel");
        assert!(channels.open("a").is_none(), "a third channel is refused");
        assert!(channels.open("b").is_some(), "another session has its own count");

        drop(first);
        assert_eq!(channels.open_count("a"), 1);
        assert!(channels.open("a").is_some(), "a closed channel frees its place");
    }
}
