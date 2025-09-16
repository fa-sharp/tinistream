use fred::prelude::*;
use itertools::Itertools;
use rocket::{
    futures::StreamExt,
    request::{FromRequest, Outcome},
    Request,
};
use rocket_okapi::OpenApiFromRequest;

use crate::redis::{constants::*, util::*, StaticPool};

/// Request guard to retrieve a Redis client from the static pool. This should
/// not be used for long-running or blocking requests (use the `RedisReader` instead).
#[derive(OpenApiFromRequest)]
pub struct RedisClient {
    client: Client,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RedisClient {
    type Error = String;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let pool = req.rocket().state::<StaticPool>().expect("should exist");
        Outcome::Success(RedisClient {
            client: pool.next().clone(),
        })
    }
}

impl RedisClient {
    pub async fn stream_info(&self, key: &str) -> FredResult<(Option<String>, u64, i64)> {
        let pipeline = self.client.pipeline();
        let _: () = pipeline.hget(meta_key(key), META_STATUS_FIELD).await?;
        let _: () = pipeline.xlen(key).await?;
        let _: () = pipeline.ttl(key).await?;
        Ok(pipeline.all().await?)
    }

    /// Check if there's an active stream with the given key
    pub async fn is_active(&self, key: &str) -> FredResult<bool> {
        let status: Option<String> = self.client.hget(meta_key(key), META_STATUS_FIELD).await?;
        Ok(status.map_or(false, |s| s == StreamStatus::Active.as_str()))
    }

    /// Start a new stream with the given key by writing a `start` entry and setting the expiration.
    /// Deletes any old stream at the same key (make sure to check for an active stream beforehand).
    /// Returns the ID of the start entry.
    pub async fn start_stream(&self, key: &str, ttl: u32) -> FredResult<String> {
        let meta_key = meta_key(key);
        let trx = self.client.multi();
        let _: () = trx.del(&[key, &meta_key]).await?;
        let _: () = trx.xadd(key, false, None, "*", START_ENTRY).await?;
        let _: () = trx.expire(key, ttl.into(), None).await?;
        let _: () = trx.hset(&meta_key, META_ACTIVE).await?;
        let _: () = trx.expire(&meta_key, ttl.into(), None).await?;
        let mut responses: Vec<Value> = trx.exec(true).await?;

        Ok(responses.swap_remove(1).convert()?)
    }

    /// Writes a single event to the stream, while checking if the stream is active.
    /// Returns the ID of the written event, or `None` if the stream is not active.
    pub async fn write_event(
        &self,
        key: &str,
        event: Vec<(&str, &str)>,
    ) -> FredResult<Option<String>> {
        let trx = self.client.multi();
        let _: () = trx.hget(meta_key(key), META_STATUS_FIELD).await?;
        let _: () = trx.xadd(key, true, XADD_CAP, "*", event).await?;
        let (status, id): (Option<String>, Option<String>) = trx.exec(true).await?;

        if status.is_some_and(|s| s == StreamStatus::Active.as_str()) {
            Ok(id)
        } else {
            Ok(None)
        }
    }

    /// Write multiple events to the stream.
    /// Returns the IDs of the written events.
    pub async fn write_events(
        &self,
        key: &str,
        events: Vec<Vec<(&str, &str)>>,
    ) -> FredResult<Vec<String>> {
        let trx = self.client.multi();
        for event in events {
            let _: () = trx.xadd(key, true, XADD_CAP, "*", event).await?;
        }
        trx.exec(true).await
    }

    /// Mark the stream as ended.
    pub async fn end_stream(&self, key: &str) -> FredResult<()> {
        let end_entry = (EVENT_KEY, END);
        let trx = self.client.multi();
        let _: () = trx.xadd(key, true, None, "*", end_entry).await?;
        let _: () = trx.hset(meta_key(key), META_ENDED).await?;
        trx.exec(true).await
    }

    /// Mark the stream as cancelled.
    pub async fn cancel_stream(&self, key: &str) -> FredResult<()> {
        let cancel_entry = (EVENT_KEY, CANCEL);
        let trx = self.client.multi();
        let _: () = trx.xadd(key, true, None, "*", cancel_entry).await?;
        let _: () = trx.hset(meta_key(key), META_CANCELLED).await?;
        trx.exec(true).await
    }

    /// Get the ID, length, and TTL of all active streams matching the given pattern.
    pub async fn scan_streams(&self, pattern: &str) -> FredResult<Vec<(String, u64, i64)>> {
        use fred::types::scan::{ScanType, Scanner};
        const PAGE_COUNT: u32 = 50;

        // Scan for metadata keys matching the pattern
        let meta_pattern = meta_key(pattern);
        let mut stream_keys: Vec<(String, String)> = Vec::with_capacity(PAGE_COUNT as usize);
        let mut scan_stream =
            self.client
                .scan(meta_pattern, Some(PAGE_COUNT), Some(ScanType::Hash));
        while let Some(page) = scan_stream.next().await {
            let meta_keys = page?.take_results().unwrap_or_default();
            stream_keys.extend(meta_keys.into_iter().filter_map(|meta_key| {
                let meta_key_str = meta_key.into_string()?;
                let key = &meta_key_str[META_PREFIX.len()..];
                Some((key.to_owned(), meta_key_str))
            }));
        }

        // Get status, length, and TTL of each stream
        let pipeline = self.client.pipeline();
        for (key, meta_key) in &stream_keys {
            let _: () = pipeline.hget(meta_key, META_STATUS_FIELD).await?;
            let _: () = pipeline.xlen(key).await?;
            let _: () = pipeline.ttl(key).await?;
        }
        let stream_info: Vec<Value> = pipeline.all().await?;

        // Filter active streams
        let active_streams = stream_keys
            .into_iter()
            .zip(stream_info.into_iter().tuples())
            .filter_map(|((key, _), (status, len, ttl))| {
                if status.as_str()? == StreamStatus::Active.as_str() {
                    Some((key.to_owned(), len.as_u64()?, ttl.as_i64()?))
                } else {
                    None
                }
            })
            .collect();

        Ok(active_streams)
    }
}
