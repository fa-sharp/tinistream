use axum::extract::FromRequestParts;

use crate::{
    error::AppError,
    redis::{RedisClient, RedisReader, RedisWriter},
    state::AppState,
};

/// Extractor: static Redis client for quick operations
pub struct StaticClient(pub RedisClient);
/// Extractor: exclusive Redis client for long-running read operations
pub struct ReaderClient(pub RedisReader);
/// Extractor: exclusive Redis client for long-running write operations
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
        let exclusive_client = state.exclusive_pool.get().await?;
        let reader = RedisReader::new(
            exclusive_client,
            state.config.client_timeout,
            state.streams(),
        );

        Ok(Self(reader))
    }
}

impl FromRequestParts<AppState> for WriterClient {
    type Rejection = AppError;
    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let exclusive_client = state.exclusive_pool.get().await?;
        let writer = RedisWriter::new(
            exclusive_client,
            state.config.max_stream_len,
            state.streams(),
        );

        Ok(Self(writer))
    }
}
