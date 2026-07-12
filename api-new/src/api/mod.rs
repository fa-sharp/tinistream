use std::sync::Arc;

use aide::{
    openapi::{OpenApi, Server},
    swagger::Swagger,
};
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
            .nest("/info", info::routes())
            .nest("/event", ingest::routes())
            .nest("/stream", stream::routes())
            // protect all previous routes with API key
            .layer(middleware::from_extractor_with_state::<ApiKey, AppState>(
                app.state().clone(),
            ))
            .nest("/client", client::routes().into())
            .nest("/health", health::routes())
            // build & serve OpenAPI docs
            .finish_api_with(&mut openapi, |mut doc| {
                doc = doc.title("Tinistream API").server(Server {
                    url: BASE_PATH.to_owned(),
                    ..Default::default()
                });
                for tag in ["info", "ingest", "stream"] {
                    doc = doc.tag(aide::openapi::Tag {
                        name: tag.to_owned(),
                        ..Default::default()
                    });
                }
                doc
            })
            .route("/docs/openapi.json", {
                let openapi_json = serde_json::to_string(&openapi)?;
                get(openapi_route).layer(Extension(Arc::<str>::from(openapi_json)))
            })
            .route("/docs", {
                let swagger = Swagger::new(format!("{BASE_PATH}/docs/openapi.json"))
                    .with_title("Tinistream API documentation");
                get(swagger.axum_handler())
            });

        Ok(router.nest(BASE_PATH, api_routes))
    })
}

async fn openapi_route(Extension(docs): Extension<Arc<str>>) -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/json")],
        (*docs).to_owned(),
    )
}
