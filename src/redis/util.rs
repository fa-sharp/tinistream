//! Shared utilities for Redis streams

use super::constants::*;

/// Checks if the event is an ending event (`end` or `cancel`)
pub fn is_end_event((_id, data): &RedisEntry) -> bool {
    data.get(EVENT_KEY)
        .is_some_and(|t| *t == END || *t == CANCEL)
}

/// Get the URL for streaming SSE events from the given Redis stream
pub fn stream_sse_url(key: &str, base_url: &str) -> String {
    format!("{base_url}/api/client/sse?key={}", urlencoding::encode(key))
}
