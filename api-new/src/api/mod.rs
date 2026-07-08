use axum::middleware;
use axum_plugin::AdHocPlugin;

use crate::{config::AppConfig, extractors::ApiKey, state::AppState};

pub mod client;
pub mod health;
pub mod hello;

/// Adds all API routes to the server under `/api`
pub fn plugin() -> AdHocPlugin<AppState, AppConfig> {
    AdHocPlugin::named("API routes").on_setup(|app, router: axum::Router<AppState>| {
        let api_routes = axum::Router::new()
            .nest("/hello", hello::routes())
            // protect all previous routes with API key
            .layer(middleware::from_extractor_with_state::<ApiKey, AppState>(
                app.state().clone(),
            ))
            .nest("/client", client::routes())
            .nest("/health", health::routes());

        Ok(router.nest("/api", api_routes))
    })
}
