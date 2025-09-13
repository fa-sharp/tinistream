pub mod api;
pub mod auth;
pub mod config;
pub mod crypto;
pub mod errors;
pub mod redis;

use rocket::{fairing::AdHoc, get, routes};

use crate::{
    config::{get_config_provider, AppConfig},
    crypto::setup_encryption,
    errors::get_catchers,
    redis::setup_redis,
};

/// Build the rocket server, load configuration and routes, prepare for launch
pub fn build_rocket() -> rocket::Rocket<rocket::Build> {
    let mut rocket = rocket::custom(get_config_provider())
        .attach(AdHoc::config::<AppConfig>())
        .attach(setup_redis())
        .attach(setup_encryption())
        .register("/", get_catchers())
        .mount("/", routes![health]);

    // Mount API routes - routes will be added here by the generate command
    rocket = rocket.mount("/api/client", api::client_routes());
    rocket = rocket.mount("/api/stream", api::stream_routes());

    rocket
}

/// Health check endpoint
#[get("/health")]
fn health() -> &'static str {
    "OK"
}
