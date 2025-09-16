use rocket::{futures::StreamExt, get, post, serde::json::Json, Route, State};
use rocket_okapi::{okapi::openapi3::OpenApi, openapi, openapi_get_routes_spec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::ext::NumericalDuration;

use crate::{
    auth::{create_client_token, ApiKeyAuth, Crypto},
    config::AppConfig,
    data::JsonStream,
    errors::ApiError,
    redis::{stream_sse_url, RedisClient, StreamStatus, DATA_KEY, EVENT_KEY},
};

pub fn get_routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        list_streams,
        create_stream,
        create_token,
        get_stream_info,
        add_events,
        add_events_json_stream,
        cancel_stream,
        end_stream
    ]
}

/// # List streams
/// List all active streams
#[openapi(tag = "Stream")]
#[get("/?<pattern>")]
async fn list_streams(
    _api_key: ApiKeyAuth,
    pattern: Option<&str>,
    redis: RedisClient,
) -> Result<Json<Vec<StreamInfo>>, ApiError> {
    let streams = redis.scan_streams(pattern.unwrap_or("*")).await?;
    let response = streams
        .into_iter()
        .map(|(key, length, ttl)| StreamInfo { key, length, ttl })
        .collect();

    Ok(Json(response))
}

/// # Get stream info
/// Get info on an active stream
#[openapi(tag = "Stream")]
#[get("/info?<key>")]
async fn get_stream_info(
    _api_key: ApiKeyAuth,
    key: &str,
    redis: RedisClient,
) -> Result<Json<StreamInfo>, ApiError> {
    let (status, length, ttl) = redis.stream_info(key).await?;
    if status.is_none_or(|s| s != StreamStatus::Active.as_str()) {
        return Err(ApiError::ActiveStreamNotFound);
    }

    Ok(Json(StreamInfo {
        key: key.to_owned(),
        length,
        ttl,
    }))
}

/// # Create stream
/// Create a new stream, and get a client URL and token to connect to the stream
#[openapi(tag = "Stream")]
#[post("/", data = "<input>")]
async fn create_stream(
    _api_key: ApiKeyAuth,
    input: Json<StreamRequest>,
    redis: RedisClient,
    crypto: &State<Crypto>,
    config: &State<AppConfig>,
) -> Result<Json<StreamAccessResponse>, ApiError> {
    if redis.is_active(&input.key).await? {
        return Err(ApiError::ExistingStream);
    }
    redis.start_stream(&input.key, config.ttl).await?;

    let url = stream_sse_url(&input.key, &config.server_address);
    let plaintext_token = create_client_token(&input.key, 10.minutes());
    let token = crypto.encrypt_base64(&plaintext_token)?;

    Ok(Json(StreamAccessResponse { url, token }))
}

/// # Create stream token
/// Create a new client token to connect to a stream
#[openapi(tag = "Stream")]
#[post("/token", data = "<input>")]
async fn create_token(
    _api_key: ApiKeyAuth,
    input: Json<StreamRequest>,
    crypto: &State<Crypto>,
    config: &State<AppConfig>,
) -> Result<Json<StreamAccessResponse>, ApiError> {
    let url = stream_sse_url(&input.key, &config.server_address);
    let plaintext_token = create_client_token(&input.key, 10.minutes());
    let token = crypto.encrypt_base64(&plaintext_token)?;

    Ok(Json(StreamAccessResponse { url, token }))
}

/// # Add events
/// Add events to a stream
#[openapi(tag = "Stream")]
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

/// # Add events JSON stream
/// Add events to a stream via a JSON stream
#[openapi(tag = "Stream")]
#[post("/add/json-stream?<key>", data = "<data>")]
async fn add_events_json_stream(
    _api_key: ApiKeyAuth,
    key: &str,
    mut data: JsonStream<'_>,
    redis: RedisClient,
) -> Result<Json<AddEventsStreamResponse>, ApiError> {
    if !redis.is_active(key).await? {
        return Err(ApiError::ActiveStreamNotFound);
    }

    let mut ids: Vec<String> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    while let Some(event) = data.stream.next().await.transpose()? {
        let mut entry = vec![(EVENT_KEY, event.event.as_str())];
        if let Some(data) = event.data.as_deref() {
            entry.push((DATA_KEY, data));
        }
        match redis.write_event(key, entry).await {
            Ok(Some(id)) => ids.push(id),
            Ok(None) => break, // stream ended
            Err(err) => errors.push(err.to_string()),
        }
    }

    Ok(Json(AddEventsStreamResponse { ids, errors }))
}

/// # Cancel stream
#[openapi(tag = "Stream")]
#[post("/cancel", data = "<input>")]
async fn cancel_stream(
    _api_key: ApiKeyAuth,
    input: Json<StreamRequest>,
    redis: RedisClient,
) -> Result<Json<EndStreamResponse>, ApiError> {
    if !redis.is_active(&input.key).await? {
        return Err(ApiError::ActiveStreamNotFound);
    }
    redis.cancel_stream(&input.key).await?;

    Ok(Json(EndStreamResponse {
        status: StreamStatus::Cancelled,
    }))
}

/// # End stream
#[openapi(tag = "Stream")]
#[post("/end", data = "<input>")]
async fn end_stream(
    _api_key: ApiKeyAuth,
    input: Json<StreamRequest>,
    redis: RedisClient,
) -> Result<Json<EndStreamResponse>, ApiError> {
    if !redis.is_active(&input.key).await? {
        return Err(ApiError::ActiveStreamNotFound);
    }
    redis.end_stream(&input.key).await?;

    Ok(Json(EndStreamResponse {
        status: StreamStatus::Ended,
    }))
}

/// Information about the stream
#[derive(JsonSchema, Serialize)]
pub struct StreamInfo {
    /// Key of the stream in Redis
    key: String,
    /// Number of events in the stream
    length: u64,
    /// Expiration of the stream
    ttl: i64,
}

#[derive(JsonSchema, Deserialize)]
struct StreamRequest {
    key: String,
}

#[derive(JsonSchema, Serialize, Deserialize)]
struct StreamAccessResponse {
    /// URL to connect to the stream
    url: String,
    /// Bearer token to access the stream
    token: String,
}

#[derive(JsonSchema, Deserialize)]
struct AddEventsRequest {
    /// Key of the stream to write to
    key: String,
    /// Events to add to the stream
    events: Vec<AddEvent>,
}

#[derive(JsonSchema, Serialize, Deserialize)]
pub struct AddEvent {
    /// Name/type of the event
    pub event: String,
    /// Event data
    pub data: Option<String>,
}

#[derive(JsonSchema, Serialize)]
struct AddEventsResponse {
    /// IDs of the added events
    ids: Vec<String>,
}

#[derive(JsonSchema, Serialize)]
struct AddEventsStreamResponse {
    /// IDs of the added events
    ids: Vec<String>,
    /// Errors that occurred while adding events
    errors: Vec<String>,
}

#[derive(JsonSchema, Serialize)]
struct EndStreamResponse {
    /// Status of the stream
    status: StreamStatus,
}
