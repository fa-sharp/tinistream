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
        RedisClient,
    },
};

pub fn get_routes() -> Vec<Route> {
    routes![list_streams, create_stream, add_events]
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
pub struct CreateStreamRequest {
    key: String,
}

#[derive(Serialize, Deserialize)]
pub struct CreateStreamResponse {
    key: String,
    url: String,
    token: String,
}

/// Create a new stream and write a start entry
#[post("/", data = "<input>")]
async fn create_stream(
    _api_key: ApiKeyAuth,
    input: Json<CreateStreamRequest>,
    redis: RedisClient,
    crypto: &State<Crypto>,
    config: &State<AppConfig>,
) -> Result<Json<CreateStreamResponse>, ApiError> {
    if redis.is_active(&input.key).await? {
        return Err(ApiError::ActiveStream);
    }
    redis.start_stream(&input.key, config.ttl).await?;

    let mut url = format!("{}/api/client/sse?key=", config.server_address);
    url.push_str(&urlencoding::encode(&input.key));

    let token_str = create_client_token(&input.key, 10.minutes());
    let token_encrypted = crypto.encrypt_base64(&token_str)?;

    Ok(Json(CreateStreamResponse {
        url,
        key: input.into_inner().key,
        token: token_encrypted,
    }))
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

// Delete a stream
// #[delete("/<id>")]
// async fn delete_stream(id: Uuid) -> Result<(), ApiError> {
//     // TODO: Implement database deletion
//     Ok(())
// }
