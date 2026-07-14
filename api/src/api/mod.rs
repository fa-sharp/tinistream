use aide::{openapi::OpenApi, swagger::Swagger};
use axum::{Extension, http::header, middleware, response::IntoResponse, routing::get};
use axum_plugin::AdHocPlugin;

use crate::{config::AppConfig, extractors::ApiKey, state::AppState};

pub mod client;
pub mod health;
pub mod info;
pub mod ingest;
pub mod stream;

/// Adds all API routes to the server under `/api`
pub fn plugin() -> AdHocPlugin<AppState, AppConfig> {
    AdHocPlugin::<AppState, AppConfig>::named("API routes").on_setup(|app, router| {
        const BASE_PATH: &str = "/api";
        let mut openapi = OpenApi::default();

        let api_routes = aide::axum::ApiRouter::new()
            // backend / stream management routes
            .nest(&format!("{BASE_PATH}/info"), info::routes())
            .nest(&format!("{BASE_PATH}/event"), ingest::routes())
            .nest(&format!("{BASE_PATH}/stream"), stream::routes())
            // protect all previous routes with API key
            .layer(middleware::from_extractor_with_state::<ApiKey, AppState>(
                app.state().clone(),
            ))
            // client and health routes
            .nest(&format!("{BASE_PATH}/client"), client::routes().into())
            .nest(&format!("{BASE_PATH}/health"), health::routes())
            // build & serve OpenAPI docs
            .finish_api_with(&mut openapi, |doc| {
                let api_key_scheme = aide::openapi::SecurityScheme::ApiKey {
                    location: aide::openapi::ApiKeyLocation::Header,
                    name: app.config().api_key_header.clone(),
                    description: Some(String::from("Authentication via API key")),
                    extensions: Default::default(),
                };
                doc.title("Tinistream API")
                    .version(env!("CARGO_PKG_VERSION"))
                    .security_scheme("ApiKey", api_key_scheme)
            })
            .route(&format!("{BASE_PATH}/docs/openapi.json"), {
                let openapi_json = serde_json::to_string_pretty(&openapi)?;
                get(openapi_route).layer(Extension::<&'static str>(Box::leak(
                    openapi_json.into_boxed_str(), // OK to leak, used throughout app lifetime
                )))
            })
            .route(&format!("{BASE_PATH}/docs"), {
                let swagger = Swagger::new(format!("{BASE_PATH}/docs/openapi.json"))
                    .with_title("Tinistream API documentation");
                get(swagger.axum_handler())
            });

        Ok(router.merge(api_routes))
    })
}

async fn openapi_route(Extension(openapi_json): Extension<&'static str>) -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/json")], openapi_json)
}
