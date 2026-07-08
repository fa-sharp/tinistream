use crate::redis::{constants, types::RedisEntry};

/// Get the key for the metadata associated with a given stream key/prefix
pub fn meta_key(stream_key: &str) -> String {
    [constants::META_PREFIX, stream_key].concat()
}

/// Convert stream events to a JSON array
pub fn stream_events_to_json(entries: Vec<RedisEntry>) -> String {
    let json_events = entries
        .into_iter()
        .map(RedisEntry::into_hashmap)
        .collect::<Vec<_>>();
    serde_json::to_string(&json_events).unwrap_or_default()
}
