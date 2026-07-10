use std::{convert::Infallible, time::Duration};

use axum::{
    extract::{WebSocketUpgrade, ws::Message as WsMessage},
    response::{Sse, sse::Event as SseEvent},
    routing::get,
};
use futures::{Stream, StreamExt};

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

    if is_end {
        Ok(Sse::new(prev_events_stream.map(Ok).boxed()))
    } else {
        let stream = prev_events_stream.chain(reader.stream_sse_events(&key, &last_id));
        Ok(Sse::new(stream.map(Ok).boxed()))
    }
}

async fn client_ws(
    ClientTokenAuth { key }: ClientTokenAuth,
    LastEventId(start_id): LastEventId,
    ReaderClient(reader): ReaderClient,
    ws: WebSocketUpgrade,
) -> AppResult<axum::response::Response> {
    let (prev_events, last_id, is_end) = reader.prev_json_events(&key, start_id.as_deref()).await?;
    let prev_events_stream = async_stream::stream! {
        // Slight delay needed here for initial WebSocket connection/handshake
        tokio::time::sleep(Duration::from_millis(200)).await;
        yield WsMessage::text(prev_events);
        if is_end {
            yield WsMessage::Close(None);
        }
    };

    if is_end {
        Ok(ws.on_upgrade(async |socket| {
            if let Err(err) = prev_events_stream.map(Ok).forward(socket).await {
                tracing::warn!("WebSocket client stream error: {err}");
            }
        }))
    } else {
        let stream = prev_events_stream.chain(reader.stream_ws_events(&key, &last_id));
        Ok(ws.on_upgrade(async |socket| {
            if let Err(err) = stream.map(Ok).forward(socket).await {
                tracing::warn!("WebSocket client stream error: {err}");
            }
        }))
    }
}
