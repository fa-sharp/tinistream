use axum::{
    Json,
    extract::{Query, WebSocketUpgrade},
    routing,
};
use futures::{SinkExt, Stream, StreamExt, TryStreamExt, stream::TryReadyChunksError};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    extractors::{JsonStream, WriterClient},
    redis::AddEvent,
    state::AppState,
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/add/json-stream", routing::post(json_stream))
        .route("/add/ws-stream", routing::get(ws_stream))
}

#[derive(Debug, Deserialize, JsonSchema)]
struct StreamKeyQuery {
    /// Key of the stream
    key: String,
}

async fn json_stream(
    Query(query): Query<StreamKeyQuery>,
    WriterClient(writer): WriterClient,
    JsonStream(stream): JsonStream,
) -> AppResult<Json<JsonStreamResponse>> {
    let mut stream_chunks = stream.try_ready_chunks(10);
    let mut num_events = 0;

    while let Some(read_result) = stream_chunks.next().await {
        match read_result {
            Ok(events) => {
                let entries = events.into_iter().map(AddEvent::into_entry);
                if let Some(ids) = writer.write_events(&query.key, entries).await? {
                    num_events += ids.len();
                } else {
                    return Err(AppError::bad_request("stream not active"));
                }
            }
            Err(TryReadyChunksError(events, _err)) => {
                // Try to write the successfully read events
                let entries = events.into_iter().map(AddEvent::into_entry);
                if let Some(ids) = writer.write_events(&query.key, entries).await? {
                    num_events += ids.len();
                } else {
                    return Err(AppError::bad_request("stream not active"));
                }
            }
        }
    }

    Ok(Json(JsonStreamResponse { num_events }))
}

#[derive(Debug, Serialize, JsonSchema)]
struct JsonStreamResponse {
    num_events: usize,
}

async fn ws_stream(
    Query(query): Query<StreamKeyQuery>,
    WriterClient(writer): WriterClient,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    let response = ws.on_upgrade(async move |ws| {
        use axum::extract::ws::Message;

        let (mut ws_writer, ws_reader) = ws.split();
        let mut stream_chunks = transform_ws_stream(ws_reader).try_ready_chunks(10);

        while let Some(result) = stream_chunks.next().await {
            match result {
                Ok(events) => {
                    let entries = events.into_iter().map(|e| e.into_entry());
                    match writer.write_events(&query.key, entries).await {
                        Ok(Some(ids)) => {
                            let message = format!("success: added {} events", ids.len());
                            let _ = ws_writer.send(Message::text(message)).await;
                        }
                        Ok(None) => {
                            let message = "error: stream is not active";
                            let _ = ws_writer.send(Message::text(message)).await;
                            break;
                        }
                        Err(err) => {
                            let _ = ws_writer.send(Message::text(format!("error: {err}"))).await;
                        }
                    }
                }
                Err(TryReadyChunksError(events, err)) => {
                    // Try to write the successfully read events, then send error
                    let entries = events.into_iter().map(|e| e.into_entry());
                    if let Ok(Some(ids)) = writer.write_events(&query.key, entries).await {
                        let message = format!("success: added {} events", ids.len());
                        let _ = ws_writer.send(Message::text(message)).await;
                    }

                    let _ = ws_writer.send(Message::text(format!("error: {err}"))).await;
                }
            }
        }
    });

    response
}

/// Transform the incoming WebSocket stream into events
fn transform_ws_stream(
    ws_stream: impl Stream<Item = Result<axum::extract::ws::Message, axum::Error>>,
) -> impl Stream<Item = Result<AddEvent, String>> {
    tokio_stream::StreamExt::filter_map(ws_stream, |msg_result| match msg_result {
        Ok(message) => match message {
            axum::extract::ws::Message::Text(text) => {
                match serde_json::from_str::<AddEvent>(&text) {
                    Ok(event) => Some(Ok(event)),
                    Err(err) => Some(Err(err.to_string())),
                }
            }
            _ => None,
        },
        Err(err) => Some(Err(err.to_string())),
    })
}
