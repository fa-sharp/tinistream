use axum::{
    extract::{FromRequestParts, Query},
    http::header,
};
use serde::Deserialize;

use crate::{error::AppError, state::AppState};

/// Validate the client token and get the stream key it can access
pub struct ClientTokenAuth {
    key: String,
}

impl FromRequestParts<AppState> for ClientTokenAuth {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Extract the query
        let Query(query) = Query::<ClientTokenQuery>::try_from_uri(&parts.uri)
            .map_err(|_| AppError::bad_request("invalid query"))?;

        // Try to get token from the Authorization header, or from the 'token' query
        let bearer_token = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|val| val.to_str().ok())
            .and_then(|val| val.strip_prefix("Bearer "));
        let token = bearer_token
            .or(query.token.as_deref())
            .ok_or_else(|| AppError::unauthorized("missing token"))?;

        // Validate the token
        match state.client_tokens().validate(token, &query.key) {
            Ok(_) => Ok(Self { key: query.key }),
            Err(err) => Err(AppError::unauthorized(err.to_string())),
        }
    }
}

#[derive(Deserialize)]
struct ClientTokenQuery {
    key: String,
    token: Option<String>,
}
