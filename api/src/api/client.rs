use std::pin::Pin;

use rocket::{
    futures::{self, Stream, StreamExt},
    get,
    response::stream::{Event, EventStream},
    Route,
};
use rocket_okapi::{okapi::openapi3::OpenApi, openapi, openapi_get_routes_spec};

use crate::{
    auth::ClientTokenAuth,
    errors::ApiError,
    redis::{LastEventIdHeader, RedisReader},
};

pub fn get_routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![connect_sse]
}

/// # Connect SSE stream
/// Connect to a stream and receive SSE events
#[openapi(tag = "Client")]
#[get("/sse?<key>")]
async fn connect_sse(
    _token: ClientTokenAuth,
    key: &str,
    start_id: Option<LastEventIdHeader>,
    reader: RedisReader,
) -> Result<EventStream<Pin<Box<dyn Stream<Item = Event> + Send>>>, ApiError> {
    let (events, last_id, is_end) = reader.prev_sse_events(key, start_id.as_deref()).await?;
    let prev_events_stream = futures::stream::iter(events);
    if is_end {
        return Ok(EventStream::from(prev_events_stream.boxed()));
    }

    let stream = prev_events_stream.chain(reader.stream_sse_events(key, &last_id));
    Ok(EventStream::from(stream.boxed()))
}
