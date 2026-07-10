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

    /// Write events to the stream, while checking if the stream is active.
    /// Returns the IDs of the written events, or `None` if the stream is not active.
    pub async fn write_events(
        &self,
        key: &str,
        events: impl IntoIterator<Item = Vec<(&str, String)>>,
    ) -> FredResult<Option<Vec<RedisStr>>> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);

        let pipeline = self.client.pipeline();
        let _: () = pipeline
            .hget(meta_key, constants::META_STATUS_FIELD)
            .await?;
        for event in events {
            let _: () = pipeline
                .xadd(&stream_key, true, ("MAXLEN", "~", self.max_len), "*", event)
                .await?;
        }
        let responses: Vec<Option<RedisStr>> = pipeline.all().await?;

        let status = responses.first().cloned();
        let ids: Vec<_> = responses.into_iter().skip(1).flatten().collect();

        match status {
            Some(Some(status)) if *status == constants::StreamStatus::Active => Ok(Some(ids)),
            _ => {
                // Stream is not active, delete the added events
                if !ids.is_empty() {
                    let _: () = self.client.xdel(&stream_key, ids).await?;
                }
                Ok(None)
            }
        }
    }
}
