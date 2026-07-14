//! Shared constants for Redis streams

use schemars::JsonSchema;
use serde::Serialize;

/// Key of the event field in the Redis stream entry
pub const EVENT_KEY: &str = "event";
/// Key of the data field in the Redis stream entry
pub const DATA_KEY: &str = "data";

pub const START: &str = "start";
pub const CANCEL: &str = "cancel";
pub const END: &str = "end";
pub const ERROR: &str = "error";

pub const CANCEL_ENTRY: (&str, &str) = (EVENT_KEY, CANCEL);
pub const END_ENTRY: (&str, &str) = (EVENT_KEY, END);
pub const ERROR_ENTRY: (&str, &str) = (EVENT_KEY, ERROR);

pub const STREAM_PREFIX: &str = "stream:";
pub const META_PREFIX: &str = "meta:";
pub const META_STATUS_FIELD: &str = "status";

#[derive(Debug, PartialEq, Serialize, JsonSchema)]
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
impl PartialEq<str> for StreamStatus {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}
impl PartialEq<StreamStatus> for str {
    fn eq(&self, other: &StreamStatus) -> bool {
        self == other.as_str()
    }
}
