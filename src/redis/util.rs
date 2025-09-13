//! Shared utilities for Redis streams

use super::constants::*;

/// Checks if the event is an ending event (`end` or `cancel`)
pub fn is_end_event((_id, data): &RedisEntry) -> bool {
    data.get(EVENT_KEY)
        .is_some_and(|t| *t == END || *t == CANCEL)
}
