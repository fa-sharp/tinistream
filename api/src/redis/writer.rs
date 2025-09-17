use fred::prelude::{FredResult, HashesInterface, StreamsInterface, TransactionInterface};
use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};
use rocket_okapi::OpenApiFromRequest;

use crate::redis::*;

/// Request guard that retrieves a stream writer with an exclusive lock on a Redis connection, for
/// long-running write operations (e.g. for ingesting events into Redis)
#[derive(OpenApiFromRequest)]
pub struct RedisWriter {
    client: deadpool::managed::Object<ExclusiveClientManager>,
}

#[async_trait]
impl<'r> FromRequest<'r> for RedisWriter {
    type Error = ();
    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let pool = req.rocket().state::<ExclusiveClientPool>().expect("exists");
        match pool.get().await {
            Ok(client) => Outcome::Success(RedisWriter::new(client)),
            Err(err) => {
                rocket::error!("Failed to retrieve Redis client from pool: {err}");
                Outcome::Error((Status::InternalServerError, ()))
            }
        }
    }
}

impl RedisWriter {
    pub fn new(client: deadpool::managed::Object<ExclusiveClientManager>) -> Self {
        Self { client }
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

        match status {
            Some(status) if *status == StreamStatus::Active => Ok(id),
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
