use std::future::Future;

use rocket::futures::SinkExt;
use rocket_ws::{stream::DuplexStream, Message};
use serde::Serialize;
use tokio_stream::StreamExt;

use crate::{
    api::stream::AddEvent,
    redis::{RedisWriter, DATA_KEY, EVENT_KEY},
};

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum EventResponse {
    /// Successful response with the event ID
    Success { id: String },
    /// Error response with a message
    Error { message: String },
}
impl EventResponse {
    fn success(id: String) -> Self {
        EventResponse::Success { id }
    }
    fn error(message: String) -> Self {
        EventResponse::Error { message }
    }
}

/// Ingest events from a WebSocket stream and send them to Redis.
pub fn process_websocket_events(
    mut stream: DuplexStream,
    writer: RedisWriter,
    key: String,
) -> impl Future<Output = rocket_ws::result::Result<()>> {
    async move {
        while let Some(res) = stream.next().await {
            match res {
                Ok(message) => {
                    let text = match message {
                        Message::Text(text) => text,
                        Message::Close(_) => break,
                        _ => continue,
                    };
                    let ev = match serde_json::from_str::<AddEvent>(&text) {
                        Ok(event) => event,
                        Err(err) => {
                            let error = EventResponse::error(format!("Invalid JSON: {err}"));
                            let text = serde_json::to_string(&error).unwrap_or_default();
                            stream.send(Message::Text(text)).await?;
                            continue;
                        }
                    };

                    let mut entry = vec![(EVENT_KEY, ev.event.as_str())];
                    if let Some(data) = ev.data.as_deref() {
                        entry.push((DATA_KEY, data));
                    }

                    match writer.write_event(&key, entry).await {
                        Ok(Some(id)) => {
                            let text = serde_json::to_string(&EventResponse::success(id))
                                .unwrap_or_default();
                            stream.send(Message::Text(text)).await?;
                        }
                        Ok(None) => {
                            let error = EventResponse::error("Stream not active".to_owned());
                            let text = serde_json::to_string(&error).unwrap_or_default();
                            stream.send(Message::Text(text)).await?;
                            stream.send(Message::Close(None)).await?;
                        }
                        Err(err) => {
                            let error = EventResponse::error(err.to_string());
                            let text = serde_json::to_string(&error).unwrap_or_default();
                            stream.send(Message::Text(text)).await?;
                        }
                    }
                }
                Err(err) => match err {
                    rocket_ws::result::Error::ConnectionClosed => break,
                    e => {
                        rocket::warn!("unexpected WebSocket error: {e}");
                        break;
                    }
                },
            }
        }

        Ok(())
    }
}
