use rocket::{get, post, serde::json::Json, Route, State};
use rocket_okapi::{okapi::openapi3::OpenApi, openapi, openapi_get_routes_spec};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::ext::NumericalDuration;

use crate::{
    auth::{create_client_token, ApiKeyAuth},
    config::AppConfig,
    crypto::Crypto,
    errors::ApiError,
    redis::{stream_sse_url, RedisClient, DATA_KEY, EVENT_KEY},
};

pub fn get_routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![
        list_streams,
        create_stream,
        create_token,
        add_events,
        cancel_stream,
        end_stream
    ]
}

/// # List streams
/// List all active streams
///
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
) -> Result<Json<CreateStreamResponse>, ApiError> {
    if redis.is_active(&input.key).await? {
        return Err(ApiError::ExistingStream);
    }
    redis.start_stream(&input.key, config.ttl).await?;

    let url = stream_sse_url(&input.key, &config.server_address);
    let plaintext_token = create_client_token(&input.key, 10.minutes());
    let token = crypto.encrypt_base64(&plaintext_token)?;

    Ok(Json(CreateStreamResponse { url, token }))
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
) -> Result<Json<CreateStreamResponse>, ApiError> {
    let url = stream_sse_url(&input.key, &config.server_address);
    let plaintext_token = create_client_token(&input.key, 10.minutes());
    let token = crypto.encrypt_base64(&plaintext_token)?;

    Ok(Json(CreateStreamResponse { url, token }))
}

/// # Add events
/// Add events to a stream
#[openapi(tag = "Stream")]
#[post("/add", data = "<input>")]
async fn add_events(
    _api_key: ApiKeyAuth,
    input: Json<AddEventsRequest>,
    redis: RedisClient,
    config: &State<AppConfig>,
) -> Result<Json<AddEventsResponse>, ApiError> {
    if !redis.is_active(&input.key).await? {
        return Err(ApiError::ActiveStreamNotFound);
    }

    let entries = input
        .events
        .iter()
        .map(|ev| vec![(EVENT_KEY, ev.event.as_str()), (DATA_KEY, ev.data.as_str())])
        .collect::<Vec<_>>();
    let ids = redis.write_events(&input.key, entries, config.ttl).await?;

    Ok(Json(AddEventsResponse { ids }))
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

    let id = redis.cancel_stream(&input.key).await?;
    Ok(Json(EndStreamResponse { id }))
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

    let id = redis.end_stream(&input.key).await?;
    Ok(Json(EndStreamResponse { id }))
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
struct CreateStreamResponse {
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

#[derive(JsonSchema, Deserialize)]
struct AddEvent {
    /// Name/type of the event
    event: String,
    /// Event data
    data: String,
}

#[derive(JsonSchema, Serialize)]
struct AddEventsResponse {
    /// IDs of the added events
    ids: Vec<String>,
}

#[derive(JsonSchema, Serialize)]
struct EndStreamResponse {
    /// ID of the ending event
    id: String,
}
