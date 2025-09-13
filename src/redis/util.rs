//! Shared utilities for Redis streams

use std::ops::Deref;

use rocket::response;

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

/// Convert a Redis stream event into a Rocket SSE event. Expects the event data to contain
/// an "event" and "data" field.
pub fn stream_event_to_sse((id, fields): RedisEntry) -> response::stream::Event {
    let mut event: Option<String> = None;
    let mut data: Option<String> = None;
    for (key, value) in fields {
        match key.deref() {
            EVENT_KEY => event = Some((*value).to_owned()),
            DATA_KEY => data = Some(format!(" {}", value.deref())), // SSE spec: add space before data
            _ => {}
        }
    }

    response::stream::Event::data(data.unwrap_or_default())
        .event(event.unwrap_or_else(|| "unknown".into()))
        .id((*id).to_owned())
}
