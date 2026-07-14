use fred::prelude::FredResult;

use crate::redis::{
    AddEvent, ExclusiveClient, StreamService, scripts::RedisScripts, types::RedisStr,
};

/// A stream writer with an exclusive lock on a Redis connection, for
/// long-running write operations (e.g. for ingesting events into Redis)
pub struct RedisWriter {
    client: ExclusiveClient,
    max_len: u32,
    stream: StreamService,
}

impl RedisWriter {
    pub fn new(client: ExclusiveClient, max_len: u32, stream: StreamService) -> Self {
        Self {
            client,
            max_len,
            stream,
        }
    }

    /// Write events to the stream, with an atomic check if the stream is active.
    /// Returns the IDs of the written events, or `None` if the stream is not active.
    pub async fn write_events(
        &self,
        key: &str,
        events: Vec<AddEvent>,
    ) -> FredResult<Option<Vec<RedisStr>>> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);

        RedisScripts
            .write_events(&self.client, &stream_key, &meta_key, self.max_len, events)
            .await
    }
}
