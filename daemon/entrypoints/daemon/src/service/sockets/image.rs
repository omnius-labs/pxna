//! What every image socket shares: the limits, how many sockets each session has open, and the signal that closes them (Image Socket Design §3).
use std::{collections::HashMap, sync::Arc, time::Duration};

use tokio_util::sync::CancellationToken;

use crate::prelude::*;

/// The limits of one image socket and of one session's sockets.
#[derive(Debug, Clone)]
pub struct ImageSocketLimits {
    /// How many items one connection may want at the same time.
    pub max_items: usize,
    /// The size of one text message.
    pub max_text_bytes: usize,
    /// How many sockets one session may have open.
    pub max_sockets_per_session: usize,
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

impl Default for ImageSocketLimits {
    fn default() -> Self {
        Self {
            max_items: 4096,
            max_text_bytes: 256 * 1024,
            max_sockets_per_session: 8,
            max_generating: 8,
            max_checking: 16,
            max_unsent_bytes: 4 * 1024 * 1024,
            chunk_bytes: 64 * 1024,
            ping_interval: Duration::from_secs(30),
        }
    }
}

pub struct ImageSockets {
    limits: ImageSocketLimits,
    open: Mutex<HashMap<String, usize>>,
    closing: CancellationToken,
}

impl ImageSockets {
    pub fn new(limits: ImageSocketLimits) -> Arc<Self> {
        Arc::new(Self {
            limits,
            open: Mutex::new(HashMap::new()),
            closing: CancellationToken::new(),
        })
    }

    pub fn limits(&self) -> &ImageSocketLimits {
        &self.limits
    }

    /// Cancelled when the daemon shuts down, so that every socket closes with 1001 and the server can stop.
    pub fn closing(&self) -> &CancellationToken {
        &self.closing
    }

    /// Counts a socket of the session. `None` when the session already has the most it may have open.
    pub fn open(self: &Arc<Self>, session_id: &str) -> Option<SocketSlot> {
        let mut open = self.open.lock();
        let count = open.entry(session_id.to_string()).or_default();
        if *count >= self.limits.max_sockets_per_session {
            return None;
        }
        *count += 1;
        Some(SocketSlot {
            sockets: self.clone(),
            session_id: session_id.to_string(),
        })
    }

    /// How many sockets the session has open. The tests use it to check that closed sockets are not counted.
    #[cfg(test)]
    pub fn open_count(&self, session_id: &str) -> usize {
        self.open.lock().get(session_id).copied().unwrap_or(0)
    }
}

/// A socket of a session that is counted as open until this is dropped.
pub struct SocketSlot {
    sockets: Arc<ImageSockets>,
    session_id: String,
}

impl Drop for SocketSlot {
    fn drop(&mut self) {
        let mut open = self.sockets.open.lock();
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
    fn a_session_may_have_a_limited_number_of_open_sockets_test() {
        let sockets = ImageSockets::new(ImageSocketLimits {
            max_sockets_per_session: 2,
            ..ImageSocketLimits::default()
        });
        let first = sockets.open("a").expect("the first socket");
        let _second = sockets.open("a").expect("the second socket");
        assert!(sockets.open("a").is_none(), "a third socket is refused");
        assert!(sockets.open("b").is_some(), "another session has its own count");

        drop(first);
        assert_eq!(sockets.open_count("a"), 1);
        assert!(sockets.open("a").is_some(), "a closed socket frees its place");
    }
}
