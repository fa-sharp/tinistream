use rocket::{
    catch, catchers,
    response::{self, Responder},
    serde::json::Json,
    Catcher, Request,
};
use serde::Serialize;
use thiserror::Error;

use crate::crypto::CryptoError;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Redis error: {0}")]
    Redis(#[from] fred::error::Error),
    #[error("Authentication error: {0}")]
    Authentication(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Active stream at this key")]
    ActiveStream,
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Internal server error: {0}")]
    Internal(String),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
}

#[derive(Debug, Serialize)]
struct ErrorMessage {
    message: String,
}

impl ErrorMessage {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}

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

impl<'r, 'o: 'r> response::Responder<'r, 'o> for ApiError {
    fn respond_to(self, req: &'r Request<'_>) -> response::Result<'o> {
        rocket::warn!("API error: {:?}", self);

        match self {
            ApiError::Authentication(_) => {
                ApiErrorResponse::Unauthorized(Json(ErrorMessage::new("Unauthorized")))
                    .respond_to(req)
            }
            ApiError::BadRequest(error) => {
                ApiErrorResponse::BadRequest(Json(ErrorMessage::new(&error))).respond_to(req)
            }
            ApiError::ActiveStream => {
                ApiErrorResponse::BadRequest(Json(ErrorMessage::new("Active stream at this key")))
                    .respond_to(req)
            }
            ApiError::NotFound(error) => {
                ApiErrorResponse::NotFound(Json(ErrorMessage::new(&error))).respond_to(req)
            }
            ApiError::Crypto(_) => {
                ApiErrorResponse::Unauthorized(Json(ErrorMessage::new("Unauthorized")))
                    .respond_to(req)
            }
            _ => ApiErrorResponse::Server(Json(ErrorMessage::new("Internal server error")))
                .respond_to(req),
        }
    }
}

/// Default JSON error catchers
pub fn get_catchers() -> Vec<Catcher> {
    catchers![bad_request, unauthorized, not_found, server_error]
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

#[catch(500)]
fn server_error(_req: &Request) -> ApiError {
    ApiError::Internal("Internal server error".to_owned())
}
