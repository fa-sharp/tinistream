use axum::response::sse;
use fred::types::FromValue;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::redis::constants;

/// An axum SSE event
pub type SseEvent = sse::Event;
/// An axum WebSocket message
pub type WsMessage = axum::extract::ws::Message;
/// An intermediate, bytes-backed representation of a string from Redis (avoids re-allocation)
pub type RedisStr = fred::bytes_utils::Str;

/// Represents a Redis stream entry retrieved via the fred client
pub struct RedisEntry {
    pub id: RedisStr,
    fields: Vec<(RedisStr, RedisStr)>,
}
impl FromValue for RedisEntry {
    fn from_value(value: fred::prelude::Value) -> Result<Self, fred::prelude::Error> {
        let (id, fields) = value.convert()?;
        Ok(Self { id, fields })
    }
}

impl RedisEntry {
    /// Check if this entry is an ending event (i.e. event field is `end` or `cancel`)
    pub fn is_end_event(&self) -> bool {
        self.fields.iter().any(|(key, val)| {
            (&**key, &**val) == constants::END_ENTRY || (&**key, &**val) == constants::CANCEL_ENTRY
        })
    }

    /// Convert this entry into a SSE event
    pub fn into_sse_event(self) -> SseEvent {
        let (id, event, data) = self.into_parts();

        SseEvent::default()
            .id(&*id)
            .event(&*event)
            .data(data.as_deref().unwrap_or(" "))
    }

    /// Convert this entry into a JSON WebSocket message
    pub fn into_ws_message(self) -> WsMessage {
        let text = serde_json::to_string(&self.into_json()).unwrap_or_default();
        WsMessage::text(text)
    }

    /// Convert this entry into JSON (adds the entry ID as the `id` field)
    pub fn into_json(self) -> serde_json::Value {
        let (id, event, data) = self.into_parts();

        serde_json::json!({
            "id": id,
            constants::EVENT_KEY: event,
            constants::DATA_KEY: data
        })
    }

    /// Returns the id, event field, and data field
    pub fn into_parts(self) -> (RedisStr, RedisStr, Option<RedisStr>) {
        let (mut event, mut data) = (None, None);
        for (key, value) in self.fields {
            match &*key {
                constants::EVENT_KEY => event = Some(value),
                constants::DATA_KEY => data = Some(value),
                _ => {}
            }
        }

        (self.id, event.unwrap_or_else(|| "unknown".into()), data)
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

/// Event to ingest / add to the stream
#[derive(Serialize, Deserialize, JsonSchema)]
pub struct AddEvent {
    /// Name/type of the event
    pub event: String,
    /// Event data
    pub data: Option<String>,
}
