use axum::middleware;
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
        let api_routes = axum::Router::new()
            .nest("/info", info::routes())
            .nest("/event", ingest::routes())
            .nest("/stream", stream::routes())
            // protect all previous routes with API key
            .layer(middleware::from_extractor_with_state::<ApiKey, AppState>(
                app.state().clone(),
            ))
            .nest("/client", client::routes())
            .nest("/health", health::routes());

        Ok(router.nest("/api", api_routes))
    })
}
