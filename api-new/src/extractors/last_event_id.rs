use axum::extract::FromRequestParts;

/// Extractor to get the Last-Event-Id header
pub struct LastEventId(pub Option<String>);

impl<S: Send + Sync> FromRequestParts<S> for LastEventId {
    type Rejection = ();

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let last_event_id = parts
            .headers
            .get("last-event-id")
            .and_then(|h| h.to_str().ok())
            .map(String::from);

        Ok(Self(last_event_id))
    }
}
