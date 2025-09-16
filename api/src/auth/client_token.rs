use rocket::{
    http::{hyper::header::AUTHORIZATION, Status},
    outcome::{try_outcome, IntoOutcome},
    request::{FromRequest, Outcome},
    Request,
};
use rocket_okapi::request::OpenApiFromRequest;
use time::{Duration, UtcDateTime};

use super::{AuthError, Crypto};

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
    type Error = AuthError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Try to get token from the Authorization header, or from the 'token' query
        let encrypted_token = try_outcome!(req
            .headers()
            .get_one(AUTHORIZATION.as_str())
            .and_then(|value| value.strip_prefix("Bearer "))
            .or(req
                .query_fields()
                .find_map(|field| (field.name == "token").then_some(field.value)))
            .or_error((Status::Unauthorized, AuthError::MissingToken)));

        // Decrypt the client token
        let crypto = req.rocket().state::<Crypto>().expect("should be attached");
        let token = try_outcome!(crypto
            .decrypt_base64(encrypted_token)
            .or_error(Status::Unauthorized));

        // Validate the client token is not expired and has access to the requested stream key
        let stream_key = req
            .query_fields()
            .find_map(|field| (field.name == "key").then_some(field.value))
            .unwrap_or_default();
        match validate_client_token(&token, stream_key) {
            Ok(_) => Outcome::Success(ClientTokenAuth(token)),
            Err(err) => Outcome::Error((Status::Unauthorized, err)),
        }
    }
}

/// Create a plaintext client token that gives access to the given stream key
/// and is valid for the given length of time
pub fn create_client_token(key: &str, ttl: Duration) -> String {
    let unix_expires = (UtcDateTime::now() + ttl).unix_timestamp();
    format!("{unix_expires}:{key}")
}

/// Verify that the plaintext client token matches the given stream key and is not expired
fn validate_client_token(token: &str, key: &str) -> Result<(), AuthError> {
    let (unix_expires, token_key) = token.split_once(':').ok_or(AuthError::InvalidToken)?;
    let expiration = UtcDateTime::from_unix_timestamp(unix_expires.parse().unwrap_or_default())
        .map_err(|_| AuthError::InvalidToken)?;
    if expiration < UtcDateTime::now() {
        return Err(AuthError::ExpiredToken);
    }
    if token_key != key {
        return Err(AuthError::PermissionDenied);
    }

    Ok(())
}

/// OpenAPI docs for the client token
impl<'r> OpenApiFromRequest<'r> for ClientTokenAuth {
    fn from_request_input(
        _gen: &mut rocket_okapi::r#gen::OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<rocket_okapi::request::RequestHeaderInput> {
        use rocket_okapi::{okapi::openapi3, request::RequestHeaderInput};

        let security_scheme = openapi3::SecurityScheme {
            description: Some("Provide client token as a Bearer token.".to_owned()),
            data: openapi3::SecuritySchemeData::Http {
                scheme: "bearer".to_owned(),
                bearer_format: Some("bearer".to_owned()),
            },
            extensions: openapi3::Object::default(),
        };
        let mut security_req = openapi3::SecurityRequirement::new();
        security_req.insert("ClientToken".to_owned(), Vec::new());

        Ok(RequestHeaderInput::Security(
            "ClientToken".to_owned(),
            security_scheme,
            security_req,
        ))
    }
}
