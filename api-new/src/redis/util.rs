use crate::redis::{constants, types::RedisEntry};

/// Get the key for the metadata associated with a given stream key/prefix
pub fn meta_key(stream_key: &str) -> String {
    [constants::META_PREFIX, stream_key].concat()
}

/// Convert stream events to JSON
pub fn stream_events_to_json(entries: Vec<RedisEntry>) -> String {
    let entry_hashmaps = entries
        .into_iter()
        .map(RedisEntry::into_hashmap)
        .collect::<Vec<_>>();

    serde_json::to_string(&serde_json::json!({
        constants::EVENT_KEY: "prev_events",
        constants::DATA_KEY: entry_hashmaps,
    }))
    .unwrap_or_default()
}
