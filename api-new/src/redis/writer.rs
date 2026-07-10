use fred::prelude::{FredResult, HashesInterface, StreamsInterface};

use crate::redis::{ExclusiveClientManager, StreamService, constants, types::RedisStr};

/// A stream writer with an exclusive lock on a Redis connection, for
/// long-running write operations (e.g. for ingesting events into Redis)
pub struct RedisWriter {
    client: deadpool::managed::Object<ExclusiveClientManager>,
    stream: StreamService,
    max_len: u32,
}

impl RedisWriter {
    pub fn new(
        client: deadpool::managed::Object<ExclusiveClientManager>,
        max_len: u32,
        stream_service: StreamService,
    ) -> Self {
        Self {
            client,
            max_len,
            stream: stream_service,
        }
    }

    /// Writes a single event to the stream, while checking if the stream is active.
    /// Returns the ID of the written event, or `None` if the stream is not active.
    pub async fn write_event(
        &self,
        key: &str,
        event: Vec<(&str, &str)>,
    ) -> FredResult<Option<RedisStr>> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);

        let pipeline = self.client.pipeline();
        let _: () = pipeline
            .hget(meta_key, constants::META_STATUS_FIELD)
            .await?;
        let _: () = pipeline
            .xadd(stream_key, true, ("MAXLEN", "~", self.max_len), "*", event)
            .await?;
        let (status, id): (Option<RedisStr>, Option<RedisStr>) = pipeline.all().await?;

        match status {
            Some(status) if *status == constants::StreamStatus::Active => Ok(id),
            _ => {
                if let Some(id) = id {
                    // Stream is not active, delete the added event
                    let _: () = self.client.xdel(key, id).await?;
                }
                Ok(None)
            }
        }
    }
}
