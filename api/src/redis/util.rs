//! Shared utilities for Redis streams

use std::ops::Deref;

use super::constants::*;

/// Get the key/prefix for the metadata associated with a given key/prefix
pub fn meta_key(key: &str) -> String {
    [META_PREFIX, key].concat()
}

/// Checks if the event is an ending event (`end` or `cancel`)
pub fn is_end_event((_id, data): &RedisEntry) -> bool {
    data.get(EVENT_KEY)
        .is_some_and(|t| *t == END_ENTRY.1 || *t == CANCEL_ENTRY.1)
}

/// Get the URL for streaming SSE events from the given Redis stream
pub fn stream_sse_url(key: &str, base_url: &str) -> String {
    format!("{base_url}/api/client/sse?key={}", urlencoding::encode(key))
}

/// Convert a Redis stream event into a Rocket SSE event. Expects the event data to contain
/// an "event" and "data" field.
pub fn stream_event_to_sse((id, fields): RedisEntry) -> rocket::response::stream::Event {
    let (mut event, mut data) = (None, None);
    for (key, value) in fields {
        match key.deref() {
            EVENT_KEY => event = Some((*value).to_owned()),
            DATA_KEY => data = Some([" ", value.deref()].concat()), // SSE spec: add space before data
            _ => {}
        }
    }
    rocket::response::stream::Event::data(data.unwrap_or_default())
        .event(event.unwrap_or_else(|| "unknown".into()))
        .id((*id).to_owned())
}
