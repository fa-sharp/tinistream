use rocket::{
    futures::Stream,
    get,
    response::stream::{Event, EventStream},
    routes, Route,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{verify_client_token, ClientTokenAuth},
    errors::ApiError,
    redis::RedisReader,
};

#[derive(Serialize, Deserialize)]
pub struct Client {
    pub id: Uuid,
    pub name: String,
    // Add more fields as needed
}

#[derive(Deserialize)]
pub struct CreateClientRequest {
    pub name: String,
    // Add more fields as needed
}

pub fn get_routes() -> Vec<Route> {
    routes![connect_sse]
}

/// Connect to a stream and receive SSE events
#[get("/sse?<key>")]
async fn connect_sse(
    token: ClientTokenAuth,
    key: &str,
    reader: RedisReader,
) -> Result<EventStream<impl Stream<Item = Event>>, ApiError> {
    verify_client_token(&token, key)?;

    let stream = reader.stream_sse_events(key, "0-0");

    Ok(EventStream::from(stream))
}
