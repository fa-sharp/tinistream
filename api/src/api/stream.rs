use rocket::{get, post, serde::json::Json, Route, State};
use rocket_okapi::{okapi::openapi3::OpenApi, openapi, openapi_get_routes_spec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::{ext::NumericalDuration, format_description::well_known, UtcDateTime};

use crate::{
    auth::{create_client_token, ApiKeyAuth, Crypto},
    config::AppConfig,
    errors::ApiError,
    redis::*,
};

pub fn get_routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        list_streams,
        create_stream,
        create_token,
        get_stream_info,
        get_stream_events,
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
    if status.is_none_or(|s| *s != StreamStatus::Active) {
        return Err(ApiError::ActiveStreamNotFound);
    }

    Ok(Json(StreamInfo {
        key: key.to_owned(),
        length,
        ttl,
    }))
}

/// # Get stream events
/// Get all events so far in a stream
#[openapi(tag = "Stream")]
#[get("/events?<key>")]
async fn get_stream_events(
    _api_key: ApiKeyAuth,
    key: &str,
    reader: RedisReader,
) -> Result<Json<Vec<StreamEvent>>, ApiError> {
    let (prev_events, _, _) = reader.get_prev_events(key, None).await?;
    let events = prev_events
        .into_iter()
        .filter_map(|(id, mut data)| {
            let unix_millis: i64 = id.split('-').next().unwrap_or_default().parse().ok()?;
            let date_time = UtcDateTime::from_unix_timestamp(unix_millis / 1000).ok()?;
            let iso_time = date_time.format(&well_known::Rfc3339).ok()?;
            let event = StreamEvent {
                id: (*id).to_owned(),
                time: iso_time,
                event: data.remove(EVENT_KEY).as_deref().map(|e| e.to_owned())?,
                data: data.remove(DATA_KEY).as_deref().map(|d| d.to_owned()),
            };
            Some(event)
        })
        .collect();

    Ok(Json(events))
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

#[derive(JsonSchema, Serialize)]
pub struct StreamEvent {
    /// ID of the event
    id: String,
    /// Time of the event (ISO 8601 format)
    time: String,
    /// Name/type of the event
    event: String,
    /// Event data
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
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

#[derive(JsonSchema, Serialize)]
struct EndStreamResponse {
    /// Status of the stream
    status: StreamStatus,
}
