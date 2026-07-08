use fred::prelude::*;
use futures::StreamExt;
use itertools::Itertools;

use crate::redis::{constants, types::RedisStr, util};

/// Redis client from static pool. Used for quick operations like retrieving stream status and
/// initializing a stream, not long-running / blocking commands.
pub struct RedisClient<'r> {
    client: &'r Client,
    max_len: u32,
}

impl<'r> RedisClient<'r> {
    pub fn new(client: &'r Client, max_len: u32) -> Self {
        Self { client, max_len }
    }

    /// Get status, length, and TTL of a stream
    pub async fn stream_info(&self, key: &str) -> FredResult<(Option<RedisStr>, u64, i64)> {
        let pipeline = self.client.pipeline();
        let _: () = pipeline
            .hget(util::meta_key(key), constants::META_STATUS_FIELD)
            .await?;
        let _: () = pipeline.xlen(key).await?;
        let _: () = pipeline.ttl(key).await?;

        pipeline.all().await
    }

    /// Check if there's an active stream with the given key
    pub async fn is_active(&self, key: &str) -> FredResult<bool> {
        let status: Option<RedisStr> = self
            .client
            .hget(util::meta_key(key), constants::META_STATUS_FIELD)
            .await?;
        Ok(status.is_some_and(|s| *s == constants::StreamStatus::Active))
    }

    /// Start a new stream with the given key by writing a `start` entry and setting the expiration.
    /// Deletes any old stream at the same key (make sure to check for an active stream beforehand).
    /// Returns the ID of the start entry.
    pub async fn start_stream(&self, key: &str, ttl: u32) -> FredResult<RedisStr> {
        let meta_key = util::meta_key(key);

        let trx = self.client.multi();
        let _: () = trx.del(&[key, &meta_key]).await?;
        let _: () = trx
            .xadd(key, false, None, "*", constants::START_ENTRY)
            .await?;
        let _: () = trx.expire(key, ttl.into(), None).await?;
        let _: () = trx.hset(&meta_key, constants::META_ACTIVE).await?;
        let _: () = trx.expire(&meta_key, ttl.into(), None).await?;

        let mut responses: Vec<Value> = trx.exec(true).await?;
        responses.swap_remove(1).convert()
    }

    /// Write multiple events to the stream.
    /// Returns the IDs of the written events.
    pub async fn write_events(
        &self,
        key: &str,
        events: impl IntoIterator<Item = Vec<(&str, &str)>>,
    ) -> FredResult<Vec<String>> {
        let trx = self.client.multi();
        for event in events {
            let _: () = trx
                .xadd(key, true, ("MAXLEN", "~", self.max_len), "*", event)
                .await?;
        }
        trx.exec(true).await
    }

    /// Mark the stream as ended.
    pub async fn end_stream(&self, key: &str) -> FredResult<()> {
        let trx = self.client.multi();
        let _: () = trx.xadd(key, true, None, "*", constants::END_ENTRY).await?;
        let _: () = trx.hset(util::meta_key(key), constants::META_ENDED).await?;
        trx.exec(true).await
    }

    /// Mark the stream as cancelled.
    pub async fn cancel_stream(&self, key: &str) -> FredResult<()> {
        let trx = self.client.multi();
        let _: () = trx
            .xadd(key, true, None, "*", constants::CANCEL_ENTRY)
            .await?;
        let _: () = trx
            .hset(util::meta_key(key), constants::META_CANCELLED)
            .await?;
        trx.exec(true).await
    }

    /// Get the ID, length, and TTL of all active streams matching the given pattern.
    pub async fn scan_streams(&self, pattern: &str) -> FredResult<Vec<(String, u64, i64)>> {
        use fred::types::scan::{ScanType, Scanner};
        const PAGE_COUNT: u32 = 50;
        const META_PREFIX_LEN: usize = constants::META_PREFIX.len();

        // Scan for metadata keys matching the pattern
        let meta_pattern = util::meta_key(pattern);
        let mut stream_keys: Vec<(String, String)> = Vec::with_capacity(PAGE_COUNT as usize);
        let mut scan_stream =
            self.client
                .scan(meta_pattern, Some(PAGE_COUNT), Some(ScanType::Hash));
        while let Some(page) = scan_stream.next().await {
            let meta_keys = page?.take_results().unwrap_or_default();
            stream_keys.extend(meta_keys.into_iter().filter_map(|meta_key| {
                let meta_key_str = meta_key.into_string()?;
                let key = &meta_key_str[META_PREFIX_LEN..];
                Some((key.to_owned(), meta_key_str))
            }));
        }

        // Get status, length, and TTL of each stream
        let pipeline = self.client.pipeline();
        for (key, meta_key) in &stream_keys {
            let _: () = pipeline
                .hget(meta_key, constants::META_STATUS_FIELD)
                .await?;
            let _: () = pipeline.xlen(key).await?;
            let _: () = pipeline.ttl(key).await?;
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
