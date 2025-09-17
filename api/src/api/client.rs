use std::time::Duration;

use rocket::{
    async_stream,
    futures::{self, stream::BoxStream, StreamExt},
    get,
    response::stream::{Event, EventStream},
    Route,
};
use rocket_okapi::{okapi::openapi3::OpenApi, openapi, openapi_get_routes_spec};
use rocket_ws::{result::Result as WsResult, stream::MessageStream, Message as WsMessage};

use crate::{
    auth::ClientTokenAuth,
    errors::ApiError,
    redis::{LastEventIdHeader, RedisReader},
};

pub fn get_routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![connect_sse, connect_ws]
}

/// # Connect SSE
/// Connect to a stream and receive SSE events
#[openapi(tag = "Client")]
#[get("/sse?<key>")]
async fn connect_sse(
    _token: ClientTokenAuth,
    key: &str,
    start_id: Option<LastEventIdHeader>,
    reader: RedisReader,
) -> Result<EventStream<BoxStream<Event>>, ApiError> {
    let (events, last_id, is_end) = reader.prev_sse_events(key, start_id.as_deref()).await?;
    let prev_events_stream = futures::stream::iter(events);
    if is_end {
        return Ok(EventStream::from(prev_events_stream.boxed()));
    }

    let stream = prev_events_stream.chain(reader.stream_sse_events(key, &last_id));
    Ok(EventStream::from(stream.boxed()))
}

/// # Connect WebSocket
/// Connect to a stream via WebSockets and receive JSON events.
/// The first message will be an array of the previous events.
#[openapi(skip)] // TODO doesn't create a good OpenAPI doc at the moment
#[get("/ws?<key>")]
async fn connect_ws(
    _token: ClientTokenAuth,
    key: &str,
    start_id: Option<LastEventIdHeader>,
    reader: RedisReader,
    ws: rocket_ws::WebSocket,
) -> Result<MessageStream<'static, BoxStream<'static, WsResult<WsMessage>>>, ApiError> {
    let (prev_events, last_id, is_end) = reader.prev_json_events(key, start_id.as_deref()).await?;
    let prev_events_stream = async_stream::stream! {
        // Slight delay needed here for initial WebSocket connection/handshake
        tokio::time::sleep(Duration::from_millis(200)).await;
        yield Ok(WsMessage::Text(prev_events));
        if is_end {
            yield Ok(WsMessage::Close(None));
        }
    };

    if is_end {
        Ok(ws.stream(|_| prev_events_stream.boxed()))
    } else {
        let stream = prev_events_stream.chain(reader.stream_ws_events(key, &last_id));
        Ok(ws.stream(|_| stream.boxed()))
    }
}
