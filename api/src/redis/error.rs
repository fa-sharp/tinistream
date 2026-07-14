use crate::error::AppError;

/// Result type for Redis operations
pub type RedisResult<T> = Result<T, RedisError>;

/// Error while reading/writing to Redis
#[derive(Debug, thiserror::Error)]
pub enum RedisError {
    #[error("Redis client error: {0}")]
    Client(#[from] fred::prelude::Error),
    #[error("Stream not found")]
    StreamNotFound,
}

impl From<RedisError> for AppError {
    fn from(error: RedisError) -> Self {
        match error {
            RedisError::StreamNotFound => Self::not_found("stream not found"),
            err => Self::internal(err.into()),
        }
    }
}
