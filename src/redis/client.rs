use fred::{
    prelude::{Client, FredResult, KeysInterface, Pool, StreamsInterface, TransactionInterface},
    types::{
        scan::{ScanType, Scanner},
        Value,
    },
};
use itertools::Itertools;
use rocket::{
    futures::StreamExt,
    request::{FromRequest, Outcome},
    Request,
};

use crate::redis::{constants::*, util::*};

/// Request guard to retrieve a Redis client from the static pool. Should not be used for
/// long-running or blocking requests.
pub struct RedisClient {
    client: Client,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RedisClient {
    type Error = String;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let pool = req.rocket().state::<Pool>().expect("should be attached");
        Outcome::Success(RedisClient {
            client: pool.next().clone(),
        })
    }
}

impl RedisClient {
    /// Start a new stream with the given key by writing a `start` entry and setting the expiration.
    /// Deletes any old stream at the same key (make sure to check for active streams beforehand).
    /// Returns the ID of the start entry.
    pub async fn start_stream(&self, key: &str, ttl: u32) -> FredResult<String> {
        let start_entry = (EVENT_KEY, START);
        let pipeline = self.client.pipeline();
        let _: () = pipeline.del(key).await?;
        let _: () = pipeline.xadd(key, false, None, "*", start_entry).await?;
        let _: () = pipeline.expire(key, ttl.into(), None).await?;
        let (_, id, _): ((), String, ()) = pipeline.all().await?;
        Ok(id)
    }

    /// Write multiple events to the stream and update the stream's expiration.
    /// Returns the IDs of the written events.
    pub async fn write_events(
        &self,
        key: &str,
        events: Vec<Vec<(&str, &str)>>,
        ttl: u32,
    ) -> FredResult<Vec<String>> {
        let trx = self.client.multi(); // use a transaction
        for event in events {
            let _: () = trx.xadd(key, true, XADD_CAP, "*", event).await?;
        }
        let _: () = trx.expire(key, ttl.into(), None).await?;
        let mut results: Vec<String> = trx.exec(true).await?;
        results.pop(); // Remove expiration response

        Ok(results)
    }

    /// End the stream by writing an `end` event. Returns the ID of the end event.
    pub async fn end_stream(&self, key: &str) -> FredResult<String> {
        let end_entry = (EVENT_KEY, END);
        self.client.xadd(key, true, None, "*", end_entry).await
    }

    /// End the stream by writing a `cancel` event. Returns the ID of the cancel event.
    pub async fn cancel_stream(&self, key: &str) -> FredResult<String> {
        let cancel_entry = (EVENT_KEY, CANCEL);
        self.client.xadd(key, true, None, "*", cancel_entry).await
    }

    /// Check if there's an active stream with the given key, by looking up the last event.
    pub async fn is_active(&self, key: &str) -> FredResult<bool> {
        match self.last_n_events(key, 1).await? {
            Some(events) => Ok(!events.iter().any(is_end_event)),
            None => Ok(false),
        }
    }

    /// Get the ID, length, and TTL of all active streams matching the given pattern.
    pub async fn scan_streams(&self, pattern: &str) -> FredResult<Vec<(String, u64, i64)>> {
        const PAGE_COUNT: u32 = 50;

        let mut streams = Vec::with_capacity(PAGE_COUNT as usize);
        let mut scan_stream = self
            .client
            .scan(pattern, Some(PAGE_COUNT), Some(ScanType::Stream));
        while let Some(page) = scan_stream.next().await {
            let keys = page?.take_results().unwrap_or_default();
            streams.extend(keys.into_iter().filter_map(|key| Some(key.into_string()?)));
        }

        // Get last entry, length, and TTL of each stream
        let pipeline = self.client.pipeline();
        for key in &streams {
            let _: () = pipeline.xrevrange(key, "+", "-", Some(1)).await?;
            let _: () = pipeline.xlen(key).await?;
            let _: () = pipeline.ttl(key).await?;
        }
        let stream_info: Vec<Value> = pipeline.all().await?;

        // Check the last entry of each stream to determine if it's still active
        let active_streams = streams
            .into_iter()
            .zip(stream_info.into_iter().tuples())
            .filter_map(|(key, (entries, len, ttl))| {
                let entries: Vec<RedisEntry> = entries.convert().ok()?;
                (!entries.iter().any(is_end_event)).then_some((key, len.as_u64()?, ttl.as_i64()?))
            })
            .collect();

        Ok(active_streams)
    }

    /// Get the last `n` events from the stream with the given key.
    async fn last_n_events(&self, key: &str, n: u64) -> FredResult<Option<Vec<RedisEntry>>> {
        self.client.xrevrange(key, "+", "-", Some(n)).await
    }
}
