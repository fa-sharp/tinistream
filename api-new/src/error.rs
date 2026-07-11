use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::redis::ExclusiveClientPoolError;

/// Global result type that can be used for API route handlers
pub type AppResult<T> = Result<T, AppError>;

/// Global error type
#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    message: String,
    source: Option<anyhow::Error>,
}

impl From<anyhow::Error> for AppError {
    fn from(error: anyhow::Error) -> Self {
        Self::internal(error)
    }
}
impl From<ExclusiveClientPoolError> for AppError {
    fn from(error: ExclusiveClientPoolError) -> Self {
        Self::internal(anyhow::Error::from(error).context("failed to get exclusive Redis client"))
    }
}
impl From<fred::error::Error> for AppError {
    fn from(error: fred::error::Error) -> Self {
        Self::internal(anyhow::Error::from(error).context("Redis client error"))
    }
}

impl AppError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            source: None,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn unauthorized(error: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: "unauthorized".into(),
            source: Some(anyhow::anyhow!(error.into())),
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn too_many_requests() -> Self {
        Self::new(StatusCode::TOO_MANY_REQUESTS, "too many requests")
    }

    pub fn internal(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: "internal server error".to_string(),
            source: Some(error),
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    message: String,
    status: u16,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let Some(error) = self.source {
            tracing::warn!(error = ?error, "request failed");
        }

        let response = ErrorResponse {
            error: ErrorBody {
                message: self.message,
                status: self.status.as_u16(),
            },
        };

        (self.status, Json(response)).into_response()
    }
}
