use rocket::{get, post, routes, serde::json::Json, Route, State};
use serde::{Deserialize, Serialize};
use time::ext::NumericalDuration;

use crate::{
    auth::{create_client_token, ApiKeyAuth},
    config::AppConfig,
    crypto::Crypto,
    errors::ApiError,
    redis::{
        constants::{DATA_KEY, EVENT_KEY},
        util::stream_sse_url,
        RedisClient,
    },
};

pub fn get_routes() -> Vec<Route> {
    routes![
        list_streams,
        create_stream,
        create_token,
        add_events,
        cancel_stream,
        end_stream
    ]
}

#[derive(Serialize)]
pub struct Stream {
    key: String,
    length: u64,
    ttl: i64,
}

/// List all streams
#[get("/")]
async fn list_streams(
    _api_key: ApiKeyAuth,
    redis: RedisClient,
) -> Result<Json<Vec<Stream>>, ApiError> {
    let streams = redis.scan_streams("*").await?;
    let response = streams
        .into_iter()
        .map(|(key, length, ttl)| Stream { key, length, ttl })
        .collect();

    Ok(Json(response))
}

#[derive(Deserialize)]
pub struct StreamRequest {
    key: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateStreamResponse {
    url: String,
    token: String,
}

/// Create a new stream and get a URL and client token
#[post("/", data = "<input>")]
async fn create_stream(
    _api_key: ApiKeyAuth,
    input: Json<StreamRequest>,
    redis: RedisClient,
    crypto: &State<Crypto>,
    config: &State<AppConfig>,
) -> Result<Json<CreateStreamResponse>, ApiError> {
    if redis.is_active(&input.key).await? {
        return Err(ApiError::ActiveStream);
    }
    redis.start_stream(&input.key, config.ttl).await?;

    let url = stream_sse_url(&input.key, &config.server_address);
    let plaintext_token = create_client_token(&input.key, 10.minutes());
    let token = crypto.encrypt_base64(&plaintext_token)?;

    Ok(Json(CreateStreamResponse { url, token }))
}

/// Create a new client token for a stream
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

#[derive(Deserialize)]
struct AddEventsRequest {
    key: String,
    events: Vec<AddEvent>,
}

#[derive(Deserialize)]
struct AddEvent {
    event: String,
    data: String,
}

#[derive(Serialize)]
struct AddEventsResponse {
    ids: Vec<String>,
}

/// Add events to a stream
#[post("/add", data = "<input>")]
async fn add_events(
    _api_key: ApiKeyAuth,
    input: Json<AddEventsRequest>,
    redis: RedisClient,
    config: &State<AppConfig>,
) -> Result<Json<AddEventsResponse>, ApiError> {
    if !redis.is_active(&input.key).await? {
        return Err(ApiError::NotFound("No active stream".to_owned()));
    }

    let entries = input
        .events
        .iter()
        .map(|ev| vec![(EVENT_KEY, ev.event.as_str()), (DATA_KEY, ev.data.as_str())])
        .collect::<Vec<_>>();
    let ids = redis.write_events(&input.key, entries, config.ttl).await?;

    Ok(Json(AddEventsResponse { ids }))
}

#[derive(Serialize)]
struct EndStreamResponse {
    /// ID of the ending event
    id: String,
}

// Cancel a stream
#[post("/cancel", data = "<input>")]
async fn cancel_stream(
    _api_key: ApiKeyAuth,
    input: Json<StreamRequest>,
    redis: RedisClient,
) -> Result<Json<EndStreamResponse>, ApiError> {
    if !redis.is_active(&input.key).await? {
        return Err(ApiError::NotFound("No active stream".to_owned()));
    }

    let id = redis.cancel_stream(&input.key).await?;
    Ok(Json(EndStreamResponse { id }))
}

// End stream
#[post("/end", data = "<input>")]
async fn end_stream(
    _api_key: ApiKeyAuth,
    input: Json<StreamRequest>,
    redis: RedisClient,
) -> Result<Json<EndStreamResponse>, ApiError> {
    if !redis.is_active(&input.key).await? {
        return Err(ApiError::NotFound("No active stream".to_owned()));
    }

    let id = redis.end_stream(&input.key).await?;
    Ok(Json(EndStreamResponse { id }))
}
