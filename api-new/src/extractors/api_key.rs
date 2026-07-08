use axum::extract::FromRequestParts;
use subtle::ConstantTimeEq;

use crate::{error::AppError, state::AppState};

/// Extractor that ensures a valid API key was provided in request
pub struct ApiKey;

impl FromRequestParts<AppState> for ApiKey {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let provided_key = parts
            .headers
            .get(&state.config.api_key_header)
            .ok_or_else(|| AppError::unauthorized())?;

        match provided_key
            .as_bytes()
            .ct_eq(state.config.api_key.as_bytes())
            .into()
        {
            true => Ok(Self),
            false => Err(AppError::unauthorized()),
        }
    }
}
