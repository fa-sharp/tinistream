use rocket::{fairing::AdHoc, http::Header};

/// Setup CORS response headers
pub fn setup_cors() -> AdHoc {
    AdHoc::on_response("CORS headers", |_, res| {
        Box::pin(async move {
            res.set_header(Header::new("Access-Control-Allow-Origin", "*"));
            res.set_header(Header::new(
                "Access-Control-Allow-Methods",
                "GET, POST, PATCH, OPTIONS",
            ));
            res.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
        })
    })
}
