use rocket::{
    async_trait,
    http::Status,
    request::{FromRequest, Outcome},
    Request,
};
use rocket_okapi::OpenApiFromRequest;

/// Request guard to extract the `Last-Event-ID` header from the request
#[derive(OpenApiFromRequest)]
pub struct LastEventIdHeader(String);

impl std::ops::Deref for LastEventIdHeader {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for LastEventIdHeader {
    type Error = ();

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match req.headers().get_one("Last-Event-ID") {
            Some(event_id) => Outcome::Success(LastEventIdHeader(event_id.to_owned())),
            None => Outcome::Error((Status::BadRequest, ())),
        }
    }
}
