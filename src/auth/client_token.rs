use rocket::{
    http::{hyper::header::AUTHORIZATION, Status},
    outcome::{try_outcome, IntoOutcome},
    request::{FromRequest, Outcome},
    Request,
};
use time::{Duration, UtcDateTime};

use crate::{
    crypto::{Crypto, CryptoError},
    errors::ApiError,
};

/// Request guard to extract and decrypt the client token from the request (for clients
/// to read from Redis streams)
pub struct ClientTokenAuth(String);

impl std::ops::Deref for ClientTokenAuth {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ClientTokenAuth {
    type Error = CryptoError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Try to get token from the Authorization header, or from the 'token' query
        let encrypted_token = try_outcome!(req
            .headers()
            .get_one(AUTHORIZATION.as_str())
            .and_then(|value| value.strip_prefix("Bearer "))
            .or(req
                .query_fields()
                .find_map(|field| (field.name == "token").then_some(field.value)))
            .or_error((Status::Unauthorized, CryptoError::MissingToken)));

        // Decrypt the client token
        let crypto = req.rocket().state::<Crypto>().expect("should be attached");
        let token = try_outcome!(crypto
            .decrypt_base64(encrypted_token)
            .or_error(Status::Unauthorized));

        Outcome::Success(ClientTokenAuth(token))
    }
}

/// Create a plaintext client token that gives access to the given stream key
/// and is valid for the given length of time
pub fn create_client_token(key: &str, ttl: Duration) -> String {
    let unix_expires = (UtcDateTime::now() + ttl).unix_timestamp();
    format!("{unix_expires}:{key}")
}

/// Verify that the plaintext client token matches the given stream key and is not expired
pub fn verify_client_token(token: &str, key: &str) -> Result<bool, ApiError> {
    token
        .split_once(':')
        .filter(|(_, token_key)| *token_key == key)
        .and_then(|(unix_expires, _)| {
            let expiration = UtcDateTime::from_unix_timestamp(unix_expires.parse().ok()?).ok()?;
            (expiration > UtcDateTime::now()).then_some(true)
        })
        .ok_or_else(|| ApiError::Authentication("Invalid or expired token".to_owned()))
}
