use rocket::{
    futures::{FutureExt, StreamExt},
    get, post,
    serde::json::Json,
    Route,
};
use rocket_okapi::{okapi::openapi3::OpenApi, openapi, openapi_get_routes_spec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    auth::ApiKeyAuth,
    data::{process_websocket_events, JsonStream},
    errors::ApiError,
    redis::*,
};

pub fn get_routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![add_events, add_events_json_stream, add_events_websocket]
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct AddEvent {
    /// Name/type of the event
    pub event: String,
    /// Event data
    pub data: Option<String>,
}

#[derive(JsonSchema, Deserialize)]
struct AddEventsRequest {
    /// Key of the stream to write to
    key: String,
    /// Events to add to the stream
    events: Vec<AddEvent>,
}

#[derive(JsonSchema, Serialize)]
struct AddEventsResponse {
    /// IDs of the added events
    ids: Vec<String>,
}

/// # Add events
/// Add events to a stream
#[openapi(tag = "Events")]
#[post("/add", data = "<input>")]
async fn add_events(
    _api_key: ApiKeyAuth,
    input: Json<AddEventsRequest>,
    redis: RedisClient,
) -> Result<Json<AddEventsResponse>, ApiError> {
    if !redis.is_active(&input.key).await? {
        return Err(ApiError::ActiveStreamNotFound);
    }

    let entries = input
        .events
        .iter()
        .map(|ev| {
            let mut entry = vec![(EVENT_KEY, ev.event.as_str())];
            if let Some(data) = ev.data.as_deref() {
                entry.push((DATA_KEY, data));
            }
            entry
        })
        .collect::<Vec<_>>();
    let ids = redis.write_events(&input.key, entries).await?;

    Ok(Json(AddEventsResponse { ids }))
}

#[derive(JsonSchema, Serialize)]
struct AddEventsStreamResponse {
    /// IDs of the added events
    ids: Vec<String>,
    /// Errors that occurred while adding events
    errors: Vec<String>,
}

/// # Add events JSON stream
/// Add events to a stream via a JSON stream. Events are sent as newline-delimited JSON objects.
#[openapi(tag = "Events")]
#[post("/add/json-stream?<key>", data = "<data>")]
async fn add_events_json_stream(
    _api_key: ApiKeyAuth,
    key: &str,
    mut data: JsonStream<'_>,
    redis: RedisClient,
    writer: RedisWriter,
) -> Result<Json<AddEventsStreamResponse>, ApiError> {
    if !redis.is_active(key).await? {
        return Err(ApiError::ActiveStreamNotFound);
    }

    let mut ids: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();
    while let Some(res) = data.stream.next().await {
        match res {
            Ok(ev) => {
                let mut entry = vec![(EVENT_KEY, ev.event.as_str())];
                if let Some(data) = ev.data.as_deref() {
                    entry.push((DATA_KEY, data));
                }
                match writer.write_event(key, entry).await {
                    Ok(Some(id)) => ids.push(id),
                    Ok(None) => break, // stream ended
                    Err(err) => errors.push(err.to_string()),
                }
            }
            Err(err) => errors.push(err.to_string()),
        }
    }

    Ok(Json(AddEventsStreamResponse { ids, errors }))
}

/// # Add events WebSocket
/// Add events to a stream via a WebSocket connection. Each event is sent as a JSON message.
#[openapi(skip)] // TODO websocket auto-docs aren't great
#[get("/add/ws-stream?<key>")]
async fn add_events_websocket(
    _api_key: ApiKeyAuth,
    key: &str,
    ws: rocket_ws::WebSocket,
    redis: RedisClient,
    writer: RedisWriter,
) -> Result<rocket_ws::Channel<'static>, ApiError> {
    if !redis.is_active(key).await? {
        return Err(ApiError::ActiveStreamNotFound);
    }

    let key = key.to_owned();
    let channel = ws.channel(move |stream| process_websocket_events(stream, writer, key).boxed());

    Ok(channel)
}
