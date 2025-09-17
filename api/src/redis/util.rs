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

/// Get the URL for streaming WebSocket events from the given Redis stream
pub fn stream_ws_url(key: &str, base_url: &str) -> String {
    format!("{base_url}/api/client/ws?key={}", urlencoding::encode(key))
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

pub fn stream_event_to_ws(entry: RedisEntry) -> rocket_ws::result::Result<rocket_ws::Message> {
    Ok(rocket_ws::Message::Text(stream_event_to_json(entry)))
}

pub fn stream_event_to_json((id, mut fields): RedisEntry) -> String {
    fields.insert("id".into(), id);
    serde_json::to_string(&fields).unwrap_or_default()
}

pub fn stream_events_to_json(entries: Vec<RedisEntry>) -> String {
    let json_events = entries
        .into_iter()
        .map(|(id, mut fields)| {
            fields.insert("id".into(), id);
            fields
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&json_events).unwrap_or_default()
}
