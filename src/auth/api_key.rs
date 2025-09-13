use rocket::{
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};

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
        match req
            .headers()
            .get_one(API_KEY_HEADER)
            .is_some_and(|value| value == app_config.api_key)
        {
            true => Outcome::Success(ApiKeyAuth),
            false => Outcome::Error((Status::Unauthorized, "Invalid API key")),
        }
    }
}
