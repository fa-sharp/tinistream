use aide::OperationOutput;
use axum::{
    Json,
    extract::rejection::{JsonRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use schemars::JsonSchema;
use serde::Serialize;

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
impl From<fred::error::Error> for AppError {
    fn from(error: fred::error::Error) -> Self {
        Self::internal(anyhow::Error::from(error).context("Redis client error"))
    }
}
impl From<QueryRejection> for AppError {
    fn from(err: QueryRejection) -> Self {
        Self::bad_request(err.to_string())
    }
}
impl From<JsonRejection> for AppError {
    fn from(err: JsonRejection) -> Self {
        Self::bad_request(err.to_string())
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

#[derive(Debug, Serialize, JsonSchema)]
pub struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Debug, Serialize, JsonSchema)]
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

impl OperationOutput for AppError {
    type Inner = ErrorResponse;

    fn inferred_responses(
        ctx: &mut aide::generate::GenContext,
        operation: &mut aide::openapi::Operation,
    ) -> Vec<(Option<aide::openapi::StatusCode>, aide::openapi::Response)> {
        if let Some(response) = Json::<ErrorResponse>::operation_response(ctx, operation) {
            [400, 401, 404, 429, 500]
                .into_iter()
                .map(|code| {
                    let status_code = Some(aide::openapi::StatusCode::Code(code));
                    (status_code, response.clone())
                })
                .collect()
        } else {
            vec![]
        }
    }
}
