use fred::prelude::*;
use futures::StreamExt;
use itertools::Itertools;

use crate::redis::{AddEvent, StreamService, constants, scripts, types::RedisStr};

/// Redis client from static pool. Used for quick operations like retrieving stream status and
/// initializing a stream, not long-running / blocking commands.
pub struct RedisClient {
    client: Client,
    stream: StreamService,
    max_len: u32,
}

impl RedisClient {
    pub fn new(client: Client, max_len: u32, stream_service: StreamService) -> Self {
        Self {
            client,
            max_len,
            stream: stream_service,
        }
    }

    /// Get status, length, and TTL of a stream
    pub async fn stream_info(&self, key: &str) -> FredResult<(Option<RedisStr>, u64, i64)> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);

        let pipeline = self.client.pipeline();
        let _: () = pipeline
            .hget(meta_key, constants::META_STATUS_FIELD)
            .await?;
        let _: () = pipeline.xlen(&stream_key).await?;
        let _: () = pipeline.ttl(&stream_key).await?;

        pipeline.all().await
    }

    /// Check if there's an active stream with the given key
    pub async fn is_active(&self, key: &str) -> FredResult<bool> {
        let status: Option<RedisStr> = self
            .client
            .hget(self.stream.meta_key(key), constants::META_STATUS_FIELD)
            .await?;

        Ok(status.is_some_and(|s| *s == constants::StreamStatus::Active))
    }

    /// Start a new stream by writing a `start` entry and setting the expiration.
    /// Deletes any old inactive stream at the same key.
    /// Returns `None` if the stream is already active.
    pub async fn start_stream(&self, key: &str, ttl: u32) -> FredResult<Option<RedisStr>> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);

        scripts::SCRIPTS
            .start_stream(&self.client, &stream_key, &meta_key, ttl)
            .await
    }

    /// Write multiple events to the stream, with an atomic check if the stream is active.
    /// Returns the IDs of the written events, or `None` if the stream is not active.
    pub async fn write_events(
        &self,
        key: &str,
        events: Vec<AddEvent>,
    ) -> FredResult<Option<Vec<RedisStr>>> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);

        scripts::SCRIPTS
            .write_events(&self.client, &stream_key, &meta_key, self.max_len, events)
            .await
    }

    /// Mark the stream as ended. Returns `None` if the stream is not active.
    pub async fn end_stream(&self, key: &str) -> FredResult<Option<RedisStr>> {
        self.finish_stream(key, constants::StreamStatus::Ended, constants::END)
            .await
    }

    /// Mark the stream as cancelled. Returns `None` if the stream is not active.
    pub async fn cancel_stream(&self, key: &str) -> FredResult<Option<RedisStr>> {
        self.finish_stream(key, constants::StreamStatus::Cancelled, constants::CANCEL)
            .await
    }

    async fn finish_stream(
        &self,
        key: &str,
        status: constants::StreamStatus,
        event: &str,
    ) -> FredResult<Option<RedisStr>> {
        let stream_key = self.stream.stream_key(key);
        let meta_key = self.stream.meta_key(key);

        scripts::SCRIPTS
            .finish_stream(&self.client, &stream_key, &meta_key, status, event)
            .await
    }

    /// Get the ID, length, and TTL of all active streams matching the given pattern.
    pub async fn scan_streams(&self, pattern: Option<&str>) -> FredResult<Vec<(String, u64, i64)>> {
        use fred::types::scan::{ScanType, Scanner};
        const PAGE_COUNT: u32 = 50;

        // Scan for metadata keys matching the pattern
        let meta_pattern = self.stream.meta_key(pattern.unwrap_or("*"));
        let mut stream_keys: Vec<(String, String)> = Vec::with_capacity(PAGE_COUNT as usize);
        let mut scan_stream =
            self.client
                .scan(meta_pattern, Some(PAGE_COUNT), Some(ScanType::Hash));
        while let Some(page) = scan_stream.next().await {
            let meta_keys = page?.take_results().unwrap_or_default();
            stream_keys.extend(meta_keys.into_iter().filter_map(|meta_key| {
                let meta_key_str = meta_key.into_string()?;
                let key = &meta_key_str[self.stream.meta_key_prefix_len()..];
                Some((key.to_owned(), meta_key_str))
            }));
        }

        // Get status, length, and TTL of each stream
        let pipeline = self.client.pipeline();
        for (key, meta_key) in &stream_keys {
            let stream_key = self.stream.stream_key(key);
            let _: () = pipeline
                .hget(meta_key, constants::META_STATUS_FIELD)
                .await?;
            let _: () = pipeline.xlen(&stream_key).await?;
            let _: () = pipeline.ttl(&stream_key).await?;
        }
        let stream_info: Vec<Value> = pipeline.all().await?;

        // Filter active streams
        let active_streams = stream_keys
            .into_iter()
            .zip(stream_info.into_iter().tuples())
            .filter_map(|((key, _), (status, len, ttl))| {
                if *status.as_str()? == constants::StreamStatus::Active {
                    Some((key.to_owned(), len.as_u64()?, ttl.as_i64()?))
                } else {
                    None
                }
            })
            .collect();

        Ok(active_streams)
    }
}
