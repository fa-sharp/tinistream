use rocket::fairing::AdHoc;

use crate::config::get_app_config;

/// Setup CORS response headers
pub fn setup_cors() -> AdHoc {
    AdHoc::on_ignite("CORS response headers", |rocket| async {
        let app_config = get_app_config(&rocket);
        let origin_list: Option<Vec<&str>> = app_config
            .allowed_origins
            .as_ref()
            .map(|origins| origins.split(',').map(|origin| origin.trim()).collect());
        let allowed_origins = match origin_list {
            Some(origins) => rocket_cors::AllowedOrigins::some_exact(&origins),
            None => rocket_cors::AllowedOrigins::all(),
        };
        let allowed_headers = rocket_cors::AllowedHeaders::some(&["Accept", "Authorization"]);

        let cors = rocket_cors::CorsOptions::default()
            .allowed_origins(allowed_origins)
            .allowed_headers(allowed_headers)
            .allow_credentials(true)
            .to_cors()
            .expect("Failed to create CORS fairing");

        rocket.attach(cors)
    })
}
