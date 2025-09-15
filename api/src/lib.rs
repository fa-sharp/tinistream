pub mod api;
pub mod auth;
pub mod config;
pub mod crypto;
pub mod data;
pub mod errors;
pub mod openapi;
pub mod redis;

use rocket::fairing::AdHoc;
use rocket_okapi::mount_endpoints_and_merged_docs;

use crate::{
    config::{get_config_provider, AppConfig},
    crypto::setup_encryption,
    errors::get_catchers,
    openapi::get_openapi_routes,
    redis::setup_redis,
};

/// Build the rocket server, load configuration and routes, prepare for launch
pub fn build_rocket() -> rocket::Rocket<rocket::Build> {
    let mut rocket = rocket::custom(get_config_provider())
        .attach(AdHoc::config::<AppConfig>())
        .attach(setup_redis())
        .attach(setup_encryption())
        .register("/", get_catchers())
        .mount("/api/docs", get_openapi_routes());

    let openapi_settings = rocket_okapi::settings::OpenApiSettings::default();
    mount_endpoints_and_merged_docs! {
        rocket, "/", openapi_settings,
        "/api" => api::info_routes(),
        "/api/client" => api::client_routes(),
        "/api/stream" => api::stream_routes()
    };

    rocket
}
