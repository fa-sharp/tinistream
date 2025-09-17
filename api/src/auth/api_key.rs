use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};
use rocket_okapi::request::OpenApiFromRequest;

use crate::config::AppConfig;

/// Name of the header containing the API key
const API_KEY_HEADER: &str = "X-API-KEY";

/// Request guard to protect a route by the API key (for authentication by backend servers to
/// write and manage Redis streams)
#[derive(Debug)]
pub struct ApiKeyAuth;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKeyAuth {
    type Error = &'static str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let app_config = req.rocket().state::<AppConfig>().expect("should exist");
        let header = req.headers().get_one(API_KEY_HEADER);
        match header.is_some_and(|val| val == app_config.api_key) {
            true => Outcome::Success(ApiKeyAuth),
            false => Outcome::Error((Status::Unauthorized, "Invalid API key")),
        }
    }
}

/// OpenAPI docs for the API key
impl OpenApiFromRequest<'_> for ApiKeyAuth {
    fn from_request_input(
        _gen: &mut rocket_okapi::r#gen::OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> rocket_okapi::Result<rocket_okapi::request::RequestHeaderInput> {
        use rocket_okapi::{okapi::openapi3, request::RequestHeaderInput};

        let security_scheme = openapi3::SecurityScheme {
            description: Some("Provide backend API key.".to_owned()),
            data: openapi3::SecuritySchemeData::ApiKey {
                name: API_KEY_HEADER.to_owned(),
                location: "header".to_owned(),
            },
            extensions: openapi3::Object::default(),
        };
        let mut security_req = openapi3::SecurityRequirement::new();
        security_req.insert("ApiKey".to_owned(), Vec::new());

        Ok(RequestHeaderInput::Security(
            "ApiKey".to_owned(),
            security_scheme,
            security_req,
        ))
    }
}
