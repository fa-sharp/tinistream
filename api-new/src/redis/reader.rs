use std::{collections::HashMap, time::Duration};

use fred::{
    interfaces::ClientLike,
    prelude::{HashesInterface, StreamsInterface},
};
use futures::Stream;
use time::{UtcDateTime, format_description::well_known::Rfc3339};

use crate::redis::{
    ExclusiveClient, StreamService, constants,
    error::{RedisError, RedisResult},
    types::{RedisEntry, RedisStr, SseEvent, StreamEvent, WsMessage},
    util,
};

/// Stream reader with an exclusive lock on a Redis connection, for
/// long-running read operations (e.g. for streaming SSE events from Redis to clients)
pub struct RedisReader {
    client: ExclusiveClient,
    stream: StreamService,
    /// Timeout for listening for the next stream event
    client_timeout_ms: u64,
}

impl RedisReader {
    pub fn new(
        client: ExclusiveClient,
        client_timeout_secs: u32,
        stream_service: StreamService,
    ) -> Self {
        Self {
            client,
            client_timeout_ms: u64::from(client_timeout_secs) * 1000,
            stream: stream_service,
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

    /// Retrieve the previous events of the stream as stringified JSON, along with the
    /// last event ID and whether the stream has ended
    pub async fn prev_json_events(
        &self,
        key: &str,
        start_event_id: Option<&str>,
    ) -> RedisResult<(String, RedisStr, bool)> {
        let (prev_events, last_event_id, is_end) =
            self.get_prev_events(key, start_event_id).await?;
        let json_events = util::stream_entries_to_json(prev_events);

        Ok((json_events, last_event_id, is_end))
    }

    /// Retrieve the previous events of the stream in a human-readable format for returning via API
    pub async fn prev_formatted_events(&self, key: &str) -> RedisResult<Vec<StreamEvent>> {
        let (prev_entries, _, _) = self.get_prev_events(key, None).await?;
        let events = prev_entries
            .into_iter()
            .filter_map(|RedisEntry { id, mut fields }| {
                let unix_millis: i64 = id.split('-').next().unwrap_or_default().parse().ok()?;
                let date_time = UtcDateTime::from_unix_timestamp(unix_millis / 1000).ok()?;
                let event = StreamEvent {
                    id: (*id).to_owned(),
                    time: date_time.format(&Rfc3339).ok()?,
                    event: fields
                        .remove(constants::EVENT_KEY)
                        .as_deref()
                        .map(str::to_owned)?,
                    data: fields
                        .remove(constants::DATA_KEY)
                        .as_deref()
                        .map(str::to_owned),
                };
                Some(event)
            })
            .collect();

        Ok(events)
    }

    /// Returns a tuple containing the previous events in the stream, the last event ID,
    /// and a boolean indicating if the stream has already ended.
    async fn get_prev_events(
        &self,
        key: &str,
        start_event_id: Option<&str>,
    ) -> RedisResult<(Vec<RedisEntry>, RedisStr, bool)> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);
        let start_event_id = start_event_id.unwrap_or("0-0");

        let pipeline = self.client.pipeline();
        let _: () = pipeline
            .xrange(stream_key, ["(", start_event_id].concat(), "+", None)
            .await?;
        let _: () = pipeline
            .hget(meta_key, constants::META_STATUS_FIELD)
            .await?;
        let (prev_events, status): (Vec<RedisEntry>, Option<RedisStr>) = pipeline.all().await?;

        let status = status.ok_or(RedisError::StreamNotFound)?;
        let last_event_id = prev_events
            .last()
            .map(|entry| entry.id.to_owned())
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
        let stream_key = self.stream.stream_key(key);
        let mut last_event_id = RedisStr::from(last_event_id);

        async_stream::stream! {
            while let Some(res) = self.next_event(&stream_key, &last_event_id).await {
                match res {
                    Ok(event) if event.is_end_event() => {
                        yield event.into_sse_event();
                        break;
                    }
                    Ok(event) => {
                        last_event_id = event.id.clone();
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
        let stream_key = self.stream.stream_key(key);
        let mut last_event_id = RedisStr::from(last_event_id);

        async_stream::stream! {
            while let Some(res) = self.next_event(&stream_key, &last_event_id).await {
                match res {
                    Ok(event) if event.is_end_event() => {
                        yield event.into_ws_message();
                        yield WsMessage::Close(None);
                        break;
                    }
                    Ok(event) => {
                        last_event_id = event.id.clone();
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
    async fn next_event(
        &self,
        stream_key: &str,
        start_event_id: &str,
    ) -> Option<RedisResult<RedisEntry>> {
        self.xread(
            stream_key,
            start_event_id,
            Some(1),
            Some(self.client_timeout_ms),
        )
        .await
        .map(|mut entries| entries.pop()) // only reading 1 event in the command
        .transpose()
    }

    /// Friendlier typed blocking `XREAD` command
    async fn xread(
        &self,
        stream_key: &str,
        start_event_id: &str,
        count: Option<u64>,
        block: Option<u64>,
    ) -> RedisResult<Vec<RedisEntry>> {
        let (_key, events) = self
            .client
            .with_options(&fred::prelude::Options {
                timeout: Some(Duration::from_millis(self.client_timeout_ms)),
                ..Default::default()
            })
            .xread::<Option<Vec<(RedisStr, _)>>, _, _>(count, block, stream_key, start_event_id)
            .await?
            .and_then(|mut streams| streams.pop()) // only reading 1 stream in the command
            .ok_or(RedisError::StreamNotFound)?;

        Ok(events)
    }
}
