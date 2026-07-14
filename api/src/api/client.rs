use std::{convert::Infallible, time::Duration};

use axum::{
    extract::{WebSocketUpgrade, ws::Message as WsMessage},
    response::{
        Sse,
        sse::{Event as SseEvent, KeepAlive},
    },
    routing::get,
};
use futures::{SinkExt, Stream, StreamExt};

use crate::{
    error::AppResult,
    extractors::{ClientTokenAuth, LastEventId, ReaderClient},
    state::AppState,
};

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/sse", get(client_sse))
        .route("/ws", get(client_ws))
}

async fn client_sse(
    ClientTokenAuth { key }: ClientTokenAuth,
    LastEventId(start_id): LastEventId,
    ReaderClient(reader): ReaderClient,
) -> AppResult<Sse<impl Stream<Item = Result<SseEvent, Infallible>>>> {
    let (events, last_id, is_end) = reader.prev_sse_events(&key, start_id.as_deref()).await?;
    let prev_events_stream = futures::stream::iter(events);

    let keep_alive = KeepAlive::default().interval(Duration::from_secs(10));
    if is_end {
        Ok(Sse::new(prev_events_stream.map(Ok).boxed()).keep_alive(keep_alive))
    } else {
        let stream = prev_events_stream.chain(reader.stream_sse_events(&key, &last_id));
        Ok(Sse::new(stream.map(Ok).boxed()).keep_alive(keep_alive))
    }
}

async fn client_ws(
    ClientTokenAuth { key }: ClientTokenAuth,
    LastEventId(start_id): LastEventId,
    ReaderClient(reader): ReaderClient,
    ws: WebSocketUpgrade,
) -> AppResult<axum::response::Response> {
    let (prev_events, last_id, is_end) = reader.prev_json_events(&key, start_id.as_deref()).await?;

    if is_end {
        Ok(ws.on_upgrade(async |mut socket| {
            let _ = socket.send(WsMessage::text(prev_events)).await;
            let _ = socket.send(WsMessage::Close(None)).await;
        }))
    } else {
        let stream = reader.stream_ws_events(&key, &last_id);
        Ok(ws.on_upgrade(async |mut socket| {
            let _ = socket.send(WsMessage::text(prev_events)).await;

            let (mut ws_sender, mut ws_reader) = socket.split();
            let mut stream = std::pin::pin!(stream);
            loop {
                tokio::select! {
                    ws_msg = ws_reader.next() => {
                        match ws_msg {
                            Some(Ok(WsMessage::Close(_))) | None => break,
                            Some(Ok(_)) => continue,
                            Some(Err(err)) => {
                                tracing::warn!("WebSocket client read error: {err}");
                                break;
                            },
                        }
                    }
                    stream_msg = stream.next() => {
                        match stream_msg {
                            Some(WsMessage::Close(_)) | None => {
                                let _ = ws_sender.send(WsMessage::Close(None)).await;
                                break;
                            }
                            Some(msg) => {
                                if let Err(err) = ws_sender.send(msg).await {
                                    tracing::warn!("WebSocket client stream error: {err}");
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }))
    }
}
