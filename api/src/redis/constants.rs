//! Shared constants and types for Redis streams

use std::collections::HashMap;

use fred::bytes_utils::Str;

pub type RedisEntry = (Str, HashMap<Str, Str>);

pub const EVENT_KEY: &str = "event";
pub const DATA_KEY: &str = "data";

pub const START: &str = "start";
pub const CANCEL: &str = "cancel";
pub const END: &str = "end";

/// Max capacity of the Redis stream when adding new entries
pub const XADD_CAP: (&str, &str, u32) = ("MAXLEN", "~", 500);
