use rocket::{
    catch, catchers,
    response::{self, Responder},
    serde::json::Json,
    Catcher, Request,
};
use rocket_okapi::response::OpenApiResponderInner;
use schemars::JsonSchema;
use serde::Serialize;
use thiserror::Error;

use crate::crypto::CryptoError;

/// Errors that can arise in the API
#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Redis error: {0}")]
    Redis(#[from] fred::error::Error),
    #[error("Authentication error: {0}")]
    Authentication(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Existing stream at this key")]
    ExistingStream,
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("No active stream at this key")]
    ActiveStreamNotFound,
    #[error("Internal server error: {0}")]
    Internal(String),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

/// The response type used by Rocket to serialize and send an error to the client
#[derive(Debug, Responder)]
enum ApiErrorResponse {
    #[response(status = 400, content_type = "json")]
    BadRequest(Json<ErrorMessage>),
    #[response(status = 401, content_type = "json")]
    Unauthorized(Json<ErrorMessage>),
    #[response(status = 404, content_type = "json")]
    NotFound(Json<ErrorMessage>),
    #[response(status = 500, content_type = "json")]
    Server(Json<ErrorMessage>),
}

#[derive(Debug, JsonSchema, Serialize)]
struct ErrorMessage {
    message: String,
    code: String,
}
impl ErrorMessage {
    fn new(message: String, code: &str) -> Json<Self> {
        Json(Self {
            message,
            code: code.to_owned(),
        })
    }
}

impl<'r, 'o: 'r> response::Responder<'r, 'o> for ApiError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        // Log the error with the appropraite level
        match self {
            ApiError::Internal(_) | ApiError::Redis(_) => rocket::error!("API error: {:?}", self),
            _ => rocket::info!("API error: {:?}", self),
        }
        // Turn the error into a response
        ApiErrorResponse::from(self).respond_to(req)
    }
}

impl From<ApiError> for ApiErrorResponse {
    /// Turn the API error into an error response to be sent to the client
    /// (hide details of internal errors, auth errors, etc.)
    fn from(value: ApiError) -> Self {
        match value {
            ApiError::Authentication(_) | ApiError::Crypto(_) => {
                Self::Unauthorized(ErrorMessage::new("Unauthorized".to_owned(), "unauthorized"))
            }
            ApiError::BadRequest(error) => {
                Self::BadRequest(ErrorMessage::new(error, "bad-request"))
            }
            ApiError::ExistingStream => Self::BadRequest(ErrorMessage::new(
                ApiError::ExistingStream.to_string(),
                "bad-request",
            )),
            ApiError::ActiveStreamNotFound => Self::NotFound(ErrorMessage::new(
                ApiError::ActiveStreamNotFound.to_string(),
                "no-active-stream",
            )),
            ApiError::NotFound(error) => Self::NotFound(ErrorMessage::new(error, "not-found")),
            ApiError::Redis(_) | ApiError::Internal(_) => Self::Server(ErrorMessage::new(
                "Internal server error".to_string(),
                "server-error",
            )),
        }
    }
}

/// Catch-all JSON error catchers
pub fn get_catchers() -> Vec<Catcher> {
    catchers![
        bad_request,
        unauthorized,
        not_found,
        unprocessable_entity,
        server_error
    ]
}

#[catch(400)]
fn bad_request(_req: &Request) -> ApiError {
    ApiError::BadRequest("Bad request".to_owned())
}

#[catch(401)]
fn unauthorized(_req: &Request) -> ApiError {
    ApiError::Authentication("Unauthorized".to_owned())
}

#[catch(404)]
fn not_found(_req: &Request) -> ApiError {
    ApiError::NotFound("Not found".to_owned())
}

#[catch(422)]
fn unprocessable_entity(_req: &Request) -> ApiError {
    ApiError::BadRequest("Incorrectly formatted".to_owned())
}

#[catch(500)]
fn server_error(_req: &Request) -> ApiError {
    ApiError::Internal("Internal server error".to_owned())
}

/// OpenAPI specification for API error responses
impl OpenApiResponderInner for ApiError {
    fn responses(
        gen: &mut rocket_okapi::r#gen::OpenApiGenerator,
    ) -> rocket_okapi::Result<rocket_okapi::okapi::openapi3::Responses> {
        use rocket_okapi::okapi::openapi3::{
            MediaType, RefOr, Response as OpenApiResponse, Responses,
        };

        let mut responses = schemars::Map::new();
        let mut content = schemars::Map::new();
        content.insert(
            "application/json".to_string(),
            MediaType {
                schema: Some(gen.json_schema::<ErrorMessage>()),
                ..Default::default()
            },
        );
        let response_data = vec![
            ("400", "Bad request"),
            ("401", "Unauthorized"),
            ("404", "Not found"),
            ("422", "Incorrectly formatted"),
            ("500", "Internal server error"),
        ];
        for (status, description) in response_data {
            responses.insert(
                status.to_string(),
                RefOr::Object(OpenApiResponse {
                    description: description.to_string(),
                    content: content.clone(),
                    ..Default::default()
                }),
            );
        }
        Ok(Responses {
            responses,
            ..Default::default()
        })
    }
}
