use std::collections::HashMap;

use axum::response::sse;
use fred::types::FromValue;
use schemars::JsonSchema;
use serde::Serialize;

use crate::redis::constants;

/// An axum SSE event
pub type SseEvent = sse::Event;
/// An axum WebSocket message
pub type WsMessage = axum::extract::ws::Message;
/// An intermediate, bytes-backed representation of a string from Redis (avoids re-allocation)
pub type RedisStr = fred::bytes_utils::Str;

/// Represents a Redis stream entry retrieved via the fred client
pub struct RedisEntry(pub RedisStr, pub HashMap<RedisStr, RedisStr>);

impl FromValue for RedisEntry {
    fn from_value(value: fred::prelude::Value) -> Result<Self, fred::prelude::Error> {
        let (id, fields): (RedisStr, HashMap<RedisStr, RedisStr>) = value.convert()?;
        Ok(Self(id, fields))
    }
}

impl RedisEntry {
    /// Get the entry ID
    pub fn id(&self) -> &str {
        &*self.0
    }

    /// Check if this entry is an ending event (i.e. event field is `end` or `cancel`)
    pub fn is_end_event(&self) -> bool {
        let Self(_id, fields) = self;
        fields
            .get(constants::EVENT_KEY)
            .is_some_and(|t| *t == constants::END_ENTRY.1 || *t == constants::CANCEL_ENTRY.1)
    }

    /// Convert this entry into a SSE event
    pub fn into_sse_event(self) -> SseEvent {
        let Self(id, fields) = self;

        let (mut event, mut data) = (None, None);
        for (key, value) in fields {
            match &*key {
                constants::EVENT_KEY => event = Some(value),
                constants::DATA_KEY => data = Some([" ", &value].concat()), // SSE spec: add space before data
                _ => {}
            }
        }

        SseEvent::default()
            .id(&*id)
            .event(event.as_deref().unwrap_or_else(|| "unknown"))
            .data(data.unwrap_or_default())
    }

    /// Convert this entry into a JSON WebSocket message
    pub fn into_ws_message(self) -> WsMessage {
        let hash = self.into_hashmap();
        let text = serde_json::to_string(&hash).unwrap_or_default();
        WsMessage::text(text)
    }

    /// Convert this entry into a hashmap (adds the entry `id` as a field)
    pub fn into_hashmap(self) -> HashMap<RedisStr, RedisStr> {
        let Self(id, mut fields) = self;
        fields.insert("id".into(), id);
        fields
    }
}

/// Formatted stream event
#[derive(Serialize, JsonSchema)]
pub struct StreamEvent {
    /// ID of the event
    pub id: String,
    /// Time of the event (ISO 8601 format)
    pub time: String,
    /// Name/type of the event
    pub event: String,
    /// Event data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}
