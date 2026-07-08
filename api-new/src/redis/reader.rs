use std::collections::HashMap;

use fred::prelude::{HashesInterface, StreamsInterface};
use futures::Stream;

use crate::redis::{
    ExclusiveClientManager, constants,
    error::{RedisError, RedisResult},
    types::{RedisEntry, RedisStr, SseEvent, WsMessage},
    util,
};

/// Stream reader with an exclusive lock on a Redis connection, for
/// long-running read operations (e.g. for streaming SSE events from Redis to clients)
pub struct RedisReader {
    client: deadpool::managed::Object<ExclusiveClientManager>,
    /// Timeout for listening for the next stream event
    client_timeout_ms: u64,
}

impl RedisReader {
    pub fn new(
        client: deadpool::managed::Object<ExclusiveClientManager>,
        client_timeout_secs: u32,
    ) -> Self {
        Self {
            client,
            client_timeout_ms: u64::from(client_timeout_secs) * 1000,
        }
    }

    /// Retrieve the previous events of the stream in SSE format, along with the last event ID
    /// and whether the stream has ended
    pub async fn prev_sse_events(
        &self,
        key: &str,
        start_event_id: Option<&str>,
    ) -> RedisResult<(Vec<SseEvent>, RedisStr, bool)> {
        let (prev_events, last_event_id, is_end) =
            self.get_prev_events(key, start_event_id).await?;
        let sse_events = prev_events
            .into_iter()
            .map(RedisEntry::into_sse_event)
            .collect();

        Ok((sse_events, last_event_id, is_end))
    }

    /// Retrieve the previous events of the stream as a stringified JSON array, along with the
    /// last event ID and whether the stream has ended
    pub async fn prev_json_events(
        &self,
        key: &str,
        start_event_id: Option<&str>,
    ) -> RedisResult<(String, RedisStr, bool)> {
        let (prev_events, last_event_id, is_end) =
            self.get_prev_events(key, start_event_id).await?;
        let json_events = util::stream_events_to_json(prev_events);

        Ok((json_events, last_event_id, is_end))
    }

    /// Returns a tuple containing the previous events in the stream, the last event ID,
    /// and a boolean indicating if the stream has already ended.
    async fn get_prev_events(
        &self,
        key: &str,
        start_event_id: Option<&str>,
    ) -> RedisResult<(Vec<RedisEntry>, RedisStr, bool)> {
        let start_event_id = start_event_id.unwrap_or("0-0");
        let pipeline = self.client.pipeline();
        let _: () = pipeline
            .xrange(key, ["(", start_event_id].concat(), "+", None)
            .await?;
        let _: () = pipeline
            .hget(util::meta_key(key), constants::META_STATUS_FIELD)
            .await?;
        let (prev_events, status): (Vec<RedisEntry>, Option<RedisStr>) = pipeline.all().await?;
        let status = status.ok_or(RedisError::StreamNotFound)?;

        let last_event_id = prev_events
            .last()
            .map(|RedisEntry(id, _)| id.to_owned())
            .unwrap_or_else(|| start_event_id.into());
        let is_end = *status != constants::StreamStatus::Active;

        Ok((prev_events, last_event_id, is_end))
    }

    /// Listen for new events in the Redis stream and return SSE events as a stream
    pub fn stream_sse_events(
        self,
        key: &str,
        last_event_id: &str,
    ) -> impl Stream<Item = SseEvent> + use<> {
        let key = key.to_owned();
        let mut last_event_id = last_event_id.to_owned();

        async_stream::stream! {
            while let Some(res) = self.next_event(&key, &last_event_id).await {
                match res {
                    Ok(event) if event.is_end_event() => {
                        yield event.into_sse_event();
                        break;
                    }
                    Ok(event) => {
                        last_event_id = event.id().to_owned();
                        yield event.into_sse_event();
                    }
                    Err(err) => {
                        yield SseEvent::default().data(err.to_string()).event("error");
                        break;
                    }
                }
            }
        }
    }

    /// Listen for new events in the Redis stream and return JSON-serialized WebSocket messages.
    pub fn stream_ws_events(
        self,
        key: &str,
        last_event_id: &str,
    ) -> impl Stream<Item = WsMessage> + use<> {
        let key = key.to_owned();
        let mut last_event_id = last_event_id.to_owned();

        async_stream::stream! {
            while let Some(res) = self.next_event(&key, &last_event_id).await {
                match res {
                    Ok(event) if event.is_end_event() => {
                        yield event.into_ws_message();
                        yield WsMessage::Close(None);
                        break;
                    }
                    Ok(event) => {
                        last_event_id = event.id().to_owned();
                        yield event.into_ws_message();
                    }
                    Err(err) => {
                        let error = err.to_string();
                        let error_hash = HashMap::from([constants::ERROR_ENTRY, (constants::DATA_KEY, &error)]);
                        if let Ok(error_str) = serde_json::to_string(&error_hash) {
                            yield WsMessage::text(error_str);
                        }
                        yield WsMessage::Close(None);
                        break;
                    }
                }
            }
        }
    }

    /// Wait for the next event from the given Redis stream using a blocking `xread` command.
    async fn next_event(&self, key: &str, start_event_id: &str) -> Option<RedisResult<RedisEntry>> {
        self.xread(key, start_event_id, Some(1), Some(self.client_timeout_ms))
            .await
            .map(|mut entries| entries.pop()) // only reading 1 event in the command
            .transpose()
    }

    /// Friendlier typed blocking `XREAD` command
    async fn xread(
        &self,
        key: &str,
        start_event_id: &str,
        count: Option<u64>,
        block: Option<u64>,
    ) -> RedisResult<Vec<RedisEntry>> {
        let (_key, events) = self
            .client
            .xread::<Option<Vec<(RedisStr, _)>>, _, _>(count, block, key, start_event_id)
            .await?
            .and_then(|mut streams| streams.pop()) // only reading 1 stream in the command
            .ok_or(RedisError::StreamNotFound)?;

        Ok(events)
    }
}
