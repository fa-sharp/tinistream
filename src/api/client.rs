use rocket::{
    futures::{self, Stream, StreamExt},
    get,
    response::stream::{Event, EventStream},
    routes, Route,
};

use crate::{
    auth::{verify_client_token, ClientTokenAuth},
    errors::ApiError,
    redis::{LastEventIdHeader, RedisReader},
};

pub fn get_routes() -> Vec<Route> {
    routes![connect_sse]
}

/// # Connect SSE stream
/// Connect to a stream and receive SSE events
#[get("/sse?<key>")]
async fn connect_sse(
    token: ClientTokenAuth,
    key: &str,
    start_id: Option<LastEventIdHeader>,
    reader: RedisReader,
) -> Result<EventStream<impl Stream<Item = Event>>, ApiError> {
    verify_client_token(&token, key)?;

    let (events, last_id, is_end) = reader.prev_sse_events(key, start_id.as_deref()).await?;
    let prev_events_stream = futures::stream::iter(events);
    if is_end {
        return Ok(EventStream::from(prev_events_stream.boxed()));
    }

    let stream = prev_events_stream.chain(reader.stream_sse_events(key, &last_id));
    Ok(EventStream::from(stream.boxed()))
}
