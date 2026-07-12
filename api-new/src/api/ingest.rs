use axum::{
    Json,
    extract::{Query, WebSocketUpgrade},
};
use axum_aide_macros::api_routes;
use futures::{SinkExt, Stream, StreamExt, TryStreamExt, stream::TryReadyChunksError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    extractors::{JsonStream, StaticClient, WriterClient},
    redis::{AddEvent, RedisWriter},
    state::AppState,
};

api_routes! {
    state: AppState,
    tag: "ingest",
    security: "api-key",
    POST "/add" => add_events, "Add events";
    POST "/add/json-stream" => json_stream, "Add events via JSON stream";
    GET "/add/ws-stream" => ws_stream, "Add events via WebSocket";
}

#[derive(Debug, Deserialize, JsonSchema)]
struct StreamKeyQuery {
    /// Key of the stream
    key: String,
}

#[derive(Deserialize, JsonSchema)]
struct AddEventsRequest {
    /// Key of the stream to write to
    key: String,
    /// Events to add to the stream
    events: Vec<AddEvent>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AddEventsResponse {
    num_events: usize,
}

async fn add_events(
    StaticClient(redis): StaticClient,
    Json(input): Json<AddEventsRequest>,
) -> AppResult<Json<AddEventsResponse>> {
    if !redis.is_active(&input.key).await? {
        return Err(AppError::bad_request("stream not active"));
    }
    let num_events = redis.write_events(&input.key, input.events).await?.len();

    Ok(Json(AddEventsResponse { num_events }))
}

/// Max number of streamed events to ingest at once
const INGEST_BATCH_SIZE: usize = 50;

async fn json_stream(
    Query(query): Query<StreamKeyQuery>,
    WriterClient(writer): WriterClient,
    JsonStream(stream): JsonStream,
) -> AppResult<Json<AddEventsResponse>> {
    let mut stream_chunks = stream.try_ready_chunks(INGEST_BATCH_SIZE);
    let mut num_events = 0;

    while let Some(read_result) = stream_chunks.next().await {
        match read_result {
            Ok(events) => {
                num_events += write_event_batch(&writer, &query.key, events).await?;
            }
            Err(TryReadyChunksError(events, err)) => {
                let _ = write_event_batch(&writer, &query.key, events).await?;
                return Err(AppError::bad_request(format!("invalid event(s): {err}")));
            }
        }
    }

    Ok(Json(AddEventsResponse { num_events }))
}

async fn ws_stream(
    Query(query): Query<StreamKeyQuery>,
    WriterClient(writer): WriterClient,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    let response = ws.on_upgrade(async move |ws| {
        let (mut ws_writer, ws_reader) = ws.split();
        let mut stream_chunks = transform_ws_stream(ws_reader).try_ready_chunks(INGEST_BATCH_SIZE);

        while let Some(result) = stream_chunks.next().await {
            match result {
                Ok(items) => {
                    let should_close = items.iter().any(|item| matches!(item, WsStreamItem::Close));
                    let events = items.into_iter().filter_map(WsStreamItem::into_event);

                    match write_event_batch(&writer, &query.key, events).await {
                        Ok(n) if n > 0 => {
                            let _ = send_ws_response(&mut ws_writer, WsResponse::success(n)).await;
                        }
                        Ok(_) => {}
                        Err(err) => {
                            let response = WsResponse::error(err.to_string());
                            let _ = send_ws_response(&mut ws_writer, response).await;
                        }
                    }

                    if should_close {
                        break;
                    }
                }
                Err(TryReadyChunksError(events, err)) => {
                    let should_close = events
                        .iter()
                        .any(|item| matches!(item, WsStreamItem::Close));
                    let events = events.into_iter().filter_map(WsStreamItem::into_event);
                    if let Ok(n) = write_event_batch(&writer, &query.key, events).await
                        && n > 0
                    {
                        let _ = send_ws_response(&mut ws_writer, WsResponse::success(n)).await;
                    }

                    let _ = send_ws_response(&mut ws_writer, WsResponse::error(err)).await;
                    if should_close {
                        break;
                    }
                }
            }
        }
    });

    response
}

async fn write_event_batch(
    writer: &RedisWriter,
    key: &str,
    events: impl IntoIterator<Item = AddEvent>,
) -> AppResult<usize> {
    let events: Vec<_> = events.into_iter().collect();
    if events.is_empty() {
        return Ok(0);
    }

    match writer.write_events(key, events).await? {
        Some(ids) => Ok(ids.len()),
        None => Err(AppError::bad_request("stream not active")),
    }
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
enum WsResponse {
    Success { num_events: usize },
    Error { message: String },
}
impl WsResponse {
    fn success(num_events: usize) -> Self {
        Self::Success { num_events }
    }
    fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }
}

enum WsStreamItem {
    Event(AddEvent),
    Close,
}
impl WsStreamItem {
    fn into_event(self) -> Option<AddEvent> {
        match self {
            Self::Event(event) => Some(event),
            Self::Close => None,
        }
    }
}

/// Send a WebSocket response
async fn send_ws_response<S>(ws_writer: &mut S, response: WsResponse) -> Result<(), S::Error>
where
    S: futures::Sink<axum::extract::ws::Message> + Unpin,
{
    let text = serde_json::to_string(&response).unwrap_or_default();
    ws_writer.send(axum::extract::ws::Message::text(text)).await
}

/// Transform the incoming WebSocket stream into events
fn transform_ws_stream(
    ws_stream: impl Stream<Item = Result<axum::extract::ws::Message, axum::Error>>,
) -> impl Stream<Item = Result<WsStreamItem, String>> {
    tokio_stream::StreamExt::filter_map(ws_stream, |msg_result| match msg_result {
        Ok(message) => match message {
            axum::extract::ws::Message::Text(text) => {
                match serde_json::from_str::<AddEvent>(&text) {
                    Ok(event) => Some(Ok(WsStreamItem::Event(event))),
                    Err(err) => Some(Err(err.to_string())),
                }
            }
            axum::extract::ws::Message::Binary(bytes) => {
                match serde_json::from_slice::<AddEvent>(&bytes) {
                    Ok(event) => Some(Ok(WsStreamItem::Event(event))),
                    Err(err) => Some(Err(err.to_string())),
                }
            }
            axum::extract::ws::Message::Close(_) => Some(Ok(WsStreamItem::Close)),
            _ => None,
        },
        Err(err) => Some(Err(err.to_string())),
    })
}
