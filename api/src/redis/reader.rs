use fred::{bytes_utils::Str, prelude::*};
use rocket::{
    async_stream, async_trait,
    futures::stream::Stream,
    http::Status,
    request::{FromRequest, Outcome},
    response::stream::Event as SseEvent,
    Request,
};
use rocket_okapi::OpenApiFromRequest;

use crate::{
    errors::ApiError,
    redis::{
        constants::RedisEntry,
        util::{is_end_event, stream_event_to_sse},
        ExclusiveClientManager, ExclusiveClientPool,
    },
};

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
            Err(err) => {
                rocket::error!("Failed to retrieve Redis client from pool: {err}");
                Outcome::Error((Status::InternalServerError, ()))
            }
        }
    }
}

/// Timeout in milliseconds for the blocking `xread` command.
const XREAD_BLOCK_TIMEOUT: u64 = 10_000; // 10 seconds

impl RedisReader {
    pub fn new(client: deadpool::managed::Object<ExclusiveClientManager>) -> Self {
        Self { client }
    }

    /// Retrieve the previous SSE events from the given Redis stream, taking an optional
    /// event ID to start from.
    ///
    /// Returns a tuple containing the previous events, the last event ID, and a boolean
    /// indicating if the stream has already ended.
    pub async fn prev_sse_events(
        &self,
        key: &str,
        start_event_id: Option<&str>,
    ) -> Result<(Vec<SseEvent>, Str, bool), ApiError> {
        let start_event_id = start_event_id.unwrap_or("0-0");
        let prev_events = self.xread(&key, start_event_id, None, None).await?;
        let (last_event_id, is_end) = prev_events
            .last()
            .map(|entry| (entry.0.to_owned(), is_end_event(entry)))
            .unwrap_or_else(|| (start_event_id.into(), false));
        let sse_events = prev_events
            .into_iter()
            .map(stream_event_to_sse)
            .collect::<Vec<_>>();

        Ok((sse_events, last_event_id, is_end))
    }

    /// Listen for new events in the Redis stream using blocking `xread` commands, and
    /// return a stream of SSE events.
    pub fn stream_sse_events(self, key: &str, last_event_id: &str) -> impl Stream<Item = SseEvent> {
        let key = key.to_owned();
        let mut last_event_id = last_event_id.to_owned();

        async_stream::stream! {
            while let Some(res) = self.next_event(&key, &last_event_id).await {
                match res {
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

    /// Wait for the next event from the given Redis stream using a blocking `xread` command.
    /// Returns `None` if encounters an ending event. Will return an error upon the blocking timeout.
    async fn next_event(
        &self,
        key: &str,
        start_event_id: &str,
    ) -> Option<Result<RedisEntry, ApiError>> {
        self.xread(key, start_event_id, Some(1), Some(XREAD_BLOCK_TIMEOUT))
            .await
            .map(|mut entries| entries.pop()) // only reading 1 event in the command
            .map(|entry| entry.filter(|e| !is_end_event(e))) // return `None` for ending event
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
            .ok_or_else(|| ApiError::NotFound("Stream not found".to_owned()))?;
        Ok(events)
    }
}
