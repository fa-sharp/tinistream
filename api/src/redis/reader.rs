use std::collections::HashMap;

use bytes_utils::Str;
use fred::prelude::{HashesInterface, StreamsInterface};
use rocket::{
    async_stream, async_trait,
    futures::stream::Stream,
    http::Status,
    request::{FromRequest, Outcome},
    response::stream::Event as SseEvent,
    Request,
};
use rocket_okapi::OpenApiFromRequest;
use rocket_ws::{result::Error as WsError, Message as WsMessage};

use crate::{errors::ApiError, redis::*};

/// Request guard that retrieves a stream reader with an exclusive lock on a Redis connection, for
/// long-running read operations (e.g. for streaming SSE events from Redis to clients)
#[derive(OpenApiFromRequest)]
pub struct RedisReader {
    client: deadpool::managed::Object<ExclusiveClientManager>,
}

#[async_trait]
impl<'r> FromRequest<'r> for RedisReader {
    type Error = ();
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let pool = req.rocket().state::<ExclusiveClientPool>().expect("exists");
        match pool.get().await {
            Ok(client) => Outcome::Success(RedisReader::new(client)),
            Err(err) => match err {
                deadpool::managed::PoolError::Timeout(_) => {
                    Outcome::Error((Status::TooManyRequests, ()))
                }
                _ => {
                    rocket::error!("Failed to retrieve Redis client from pool: {err}");
                    Outcome::Error((Status::InternalServerError, ()))
                }
            },
        }
    }
}

/// Timeout in milliseconds for the blocking `xread` command.
const XREAD_BLOCK_TIMEOUT: u64 = 10_000; // 10 seconds

impl RedisReader {
    pub fn new(client: deadpool::managed::Object<ExclusiveClientManager>) -> Self {
        Self { client }
    }

    /// Retrieve the previous events of the stream in SSE format
    pub async fn prev_sse_events(
        &self,
        key: &str,
        start_event_id: Option<&str>,
    ) -> Result<(Vec<SseEvent>, Str, bool), ApiError> {
        let (prev_events, last_event_id, is_end) =
            self.get_prev_events(key, start_event_id).await?;
        let sse_events = prev_events
            .into_iter()
            .map(stream_event_to_sse)
            .collect::<Vec<_>>();

        Ok((sse_events, last_event_id, is_end))
    }

    /// Retrieve the previous events of the stream in JSON format
    pub async fn prev_json_events(
        &self,
        key: &str,
        start_event_id: Option<&str>,
    ) -> Result<(String, Str, bool), ApiError> {
        let (prev_events, last_event_id, is_end) =
            self.get_prev_events(key, start_event_id).await?;
        let json_events = stream_events_to_json(prev_events);

        Ok((json_events, last_event_id, is_end))
    }

    /// Returns a tuple containing the previous events in the stream, the last event ID,
    /// and a boolean indicating if the stream has already ended.
    pub async fn get_prev_events(
        &self,
        key: &str,
        start_event_id: Option<&str>,
    ) -> Result<(Vec<RedisEntry>, Str, bool), ApiError> {
        let start_event_id = start_event_id.unwrap_or("0-0");
        let pipeline = self.client.pipeline();
        let _: () = pipeline
            .xrange(key, ["(", start_event_id].concat(), "+", None)
            .await?;
        let _: () = pipeline.hget(meta_key(key), META_STATUS_FIELD).await?;
        let (prev_events, status): (Vec<RedisEntry>, Option<String>) = pipeline.all().await?;
        let status = status.ok_or(ApiError::StreamNotFound)?;

        let last_event_id = prev_events
            .last()
            .map(|(id, _)| id.to_owned())
            .unwrap_or_else(|| start_event_id.into());
        let is_end = *status != StreamStatus::Active;

        Ok((prev_events, last_event_id, is_end))
    }

    /// Listen for new events in the Redis stream and return SSE events.
    pub fn stream_sse_events(self, key: &str, last_event_id: &str) -> impl Stream<Item = SseEvent> {
        let key = key.to_owned();
        let mut last_event_id = last_event_id.to_owned();

        async_stream::stream! {
            while let Some(res) = self.next_event(&key, &last_event_id).await {
                match res {
                    Ok(event) if is_end_event(&event) => {
                        yield stream_event_to_sse(event);
                        break;
                    }
                    Ok((id, data)) => {
                        last_event_id = (*id).to_owned();
                        yield stream_event_to_sse((id, data));
                    }
                    Err(err) => {
                        yield SseEvent::data(err.to_string()).event("error");
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
    ) -> impl Stream<Item = Result<WsMessage, WsError>> {
        let key = key.to_owned();
        let mut last_event_id = last_event_id.to_owned();

        async_stream::stream! {
            while let Some(res) = self.next_event(&key, &last_event_id).await {
                match res {
                    Ok(event) if is_end_event(&event) => {
                        yield stream_event_to_ws(event);
                        yield Ok(WsMessage::Close(None));
                        break;
                    }
                    Ok((id, data)) => {
                        last_event_id = (*id).to_owned();
                        yield stream_event_to_ws((id, data));
                    }
                    Err(err) => {
                        let error = err.to_string();
                        let error_hash = HashMap::from([ERROR_ENTRY, (DATA_KEY, &error)]);
                        if let Ok(error_str) = serde_json::to_string(&error_hash) {
                            yield Ok(WsMessage::Text(error_str));
                        }
                        yield Ok(WsMessage::Close(None));
                        break;
                    }
                }
            }
        }
    }

    /// Wait for the next event from the given Redis stream using a blocking `xread` command.
    async fn next_event(
        &self,
        key: &str,
        start_event_id: &str,
    ) -> Option<Result<RedisEntry, ApiError>> {
        self.xread(key, start_event_id, Some(1), Some(XREAD_BLOCK_TIMEOUT))
            .await
            .map(|mut entries| entries.pop()) // only reading 1 event in the command
            .transpose()
    }

    /// Friendly typed `XREAD` wrapper to read events from a Redis stream.
    async fn xread(
        &self,
        key: &str,
        start_event_id: &str,
        count: Option<u64>,
        block: Option<u64>,
    ) -> Result<Vec<RedisEntry>, ApiError> {
        let (_key, events) = self
            .client
            .xread::<Option<Vec<(Str, _)>>, _, _>(count, block, key, start_event_id)
            .await?
            .and_then(|mut streams| streams.pop()) // only reading 1 stream in the command
            .ok_or(ApiError::StreamNotFound)?;
        Ok(events)
    }
}
