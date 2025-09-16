//! Shared constants and types for Redis streams

use std::collections::HashMap;

use fred::bytes_utils::Str;
use schemars::JsonSchema;
use serde::Serialize;

/// Type representing a Redis stream entry retrieved via the fred client
pub type RedisEntry = (Str, HashMap<Str, Str>);

/// Key of the event field in the Redis stream entry
pub const EVENT_KEY: &str = "event";
/// Key of the data field in the Redis stream entry
pub const DATA_KEY: &str = "data";

pub const START_ENTRY: (&str, &str) = (EVENT_KEY, "start");
pub const CANCEL_ENTRY: (&str, &str) = (EVENT_KEY, "cancel");
pub const END_ENTRY: (&str, &str) = (EVENT_KEY, "end");

/// Max capacity of the Redis stream when adding new entries
pub const XADD_CAP: (&str, &str, u32) = ("MAXLEN", "~", 500);

pub const META_PREFIX: &str = "tinistream:meta:";
pub const META_STATUS_FIELD: &str = "status";
pub const META_ACTIVE: (&str, &str) = (META_STATUS_FIELD, StreamStatus::Active.as_str());
pub const META_CANCELLED: (&str, &str) = (META_STATUS_FIELD, StreamStatus::Cancelled.as_str());
pub const META_ENDED: (&str, &str) = (META_STATUS_FIELD, StreamStatus::Ended.as_str());

#[derive(Debug, JsonSchema, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamStatus {
    Active,
    Cancelled,
    Ended,
}
impl StreamStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            StreamStatus::Active => "active",
            StreamStatus::Cancelled => "cancelled",
            StreamStatus::Ended => "ended",
        }
    }
}
