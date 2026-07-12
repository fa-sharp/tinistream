use axum::{
    Json,
    extract::{Query, State},
};
use axum_aide_macros::api_routes;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    extractors::{ReaderClient, StaticClient},
    redis::{StreamEvent, StreamStatus},
    state::AppState,
};

api_routes! {
    state: AppState,
    tag: "stream",
    security: "api-key",
    GET "/" => list_streams, "List streams";
    GET "/info" => get_stream_info, "Get stream info";
    GET "/events" => get_stream_events, "Get stream events";
    POST "/" => create_stream, "Create stream";
    POST "/token" => create_token, "Create client token";
    POST "/cancel" => cancel_stream, "Cancel stream";
    POST "/end" => end_stream, "End stream";
}

#[derive(Debug, Deserialize, JsonSchema)]
struct StreamPatternQuery {
    /// Key prefix / pattern to search for
    pattern: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct StreamKeyQuery {
    /// Key of the stream
    key: String,
}

async fn list_streams(
    Query(query): Query<StreamPatternQuery>,
    StaticClient(redis): StaticClient,
) -> AppResult<Json<Vec<StreamInfo>>> {
    let streams = redis.scan_streams(query.pattern.as_deref()).await?;
    let response = streams
        .into_iter()
        .map(|(key, length, ttl)| StreamInfo { key, length, ttl })
        .collect();

    Ok(Json(response))
}

async fn get_stream_info(
    Query(query): Query<StreamKeyQuery>,
    StaticClient(redis): StaticClient,
) -> AppResult<Json<StreamInfo>> {
    let (status, length, ttl) = redis.stream_info(&query.key).await?;
    if status.is_none_or(|s| *s != StreamStatus::Active) {
        return Err(AppError::not_found("active stream not found"));
    }

    Ok(Json(StreamInfo {
        key: query.key.to_owned(),
        length,
        ttl,
    }))
}

async fn get_stream_events(
    Query(query): Query<StreamKeyQuery>,
    ReaderClient(reader): ReaderClient,
) -> AppResult<Json<Vec<StreamEvent>>> {
    let events = reader.prev_formatted_events(&query.key).await?;
    Ok(Json(events))
}

/// # Create stream
/// Create a new stream, and get a client URL and token to connect to the stream
async fn create_stream(
    StaticClient(redis): StaticClient,
    State(state): State<AppState>,
    Json(input): Json<StreamRequest>,
) -> AppResult<Json<StreamAccessResponse>> {
    if redis.is_active(&input.key).await? {
        return Err(AppError::bad_request("stream at this key already exists"));
    }
    let _start_id = redis
        .start_stream(&input.key, state.config.redis_ttl)
        .await?;
    let token = state
        .client_tokens()
        .create(&input.key, state.config.redis_ttl)?;
    let stream_service = state.streams();

    Ok(Json(StreamAccessResponse {
        sse_url: stream_service.sse_url(&input.key),
        ws_url: stream_service.ws_url(&input.key),
        token,
    }))
}

/// # Create stream token
/// Create a new client token for connecting to a stream
async fn create_token(
    StaticClient(redis): StaticClient,
    State(state): State<AppState>,
    Json(input): Json<StreamRequest>,
) -> AppResult<Json<StreamAccessResponse>> {
    if !redis.is_active(&input.key).await? {
        return Err(AppError::not_found("active stream not found"));
    }
    let token = state
        .client_tokens()
        .create(&input.key, state.config.redis_ttl)?;
    let stream_service = state.streams();

    Ok(Json(StreamAccessResponse {
        sse_url: stream_service.sse_url(&input.key),
        ws_url: stream_service.ws_url(&input.key),
        token,
    }))
}

/// # Cancel stream
async fn cancel_stream(
    StaticClient(redis): StaticClient,
    Json(input): Json<StreamRequest>,
) -> AppResult<Json<EndStreamResponse>> {
    if !redis.is_active(&input.key).await? {
        return Err(AppError::not_found("active stream not found"));
    }
    redis.cancel_stream(&input.key).await?;

    Ok(Json(EndStreamResponse {
        status: StreamStatus::Cancelled,
    }))
}

/// # End stream
async fn end_stream(
    StaticClient(redis): StaticClient,
    Json(input): Json<StreamRequest>,
) -> AppResult<Json<EndStreamResponse>> {
    if !redis.is_active(&input.key).await? {
        return Err(AppError::not_found("active stream not found"));
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
    /// URL for the client to connect to the stream via SSE
    sse_url: String,
    /// URL for the client to connect to the stream via WebSocket
    ws_url: String,
    /// Client token to access the stream. Can be used as a Bearer token
    /// in the Authorization header (recommended) or as the `token` query parameter.
    token: String,
}

#[derive(JsonSchema, Serialize)]
struct EndStreamResponse {
    /// Status of the stream
    status: StreamStatus,
}
