use aide::OperationIo;
use axum::extract::FromRequestParts;

use crate::{
    error::AppError,
    redis::{RedisClient, RedisReader, RedisWriter},
    state::AppState,
};

/// Extractor: static Redis client for quick operations
#[derive(OperationIo)]
pub struct StaticClient(pub RedisClient);
/// Extractor: exclusive Redis client for long-running read operations
#[derive(OperationIo)]
pub struct ReaderClient(pub RedisReader);
/// Extractor: exclusive Redis client for long-running write operations
#[derive(OperationIo)]
pub struct WriterClient(pub RedisWriter);

impl FromRequestParts<AppState> for StaticClient {
    type Rejection = ();
    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let client = RedisClient::new(
            state.static_pool.next().to_owned(),
            state.config.max_stream_len,
            state.streams(),
        );

        Ok(Self(client))
    }
}

impl FromRequestParts<AppState> for ReaderClient {
    type Rejection = AppError;
    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match state.exclusive_clients.get().await? {
            Some(client) => {
                let reader = RedisReader::new(client, state.config.client_timeout, state.streams());
                Ok(Self(reader))
            }
            None => Err(AppError::too_many_requests()),
        }
    }
}

impl FromRequestParts<AppState> for WriterClient {
    type Rejection = AppError;
    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        match state.exclusive_clients.get().await? {
            Some(client) => {
                let writer = RedisWriter::new(
                    client,
                    state.config.max_stream_len,
                    state.streams(),
                    &state.ingest_script_hash,
                );
                Ok(Self(writer))
            }
            None => Err(AppError::too_many_requests()),
        }
    }
}
