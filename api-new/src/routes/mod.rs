use axum_plugin::AdHocPlugin;

use crate::{extractors::api_key::ApiKey, state::AppState};


pub mod health;
pub mod hello;

/// Adds all API routes to the server under `/api`
pub fn plugin() -> AdHocPlugin<AppState> {
    AdHocPlugin::named("API routes").on_setup(|router, state: &AppState| {
        let api_routes = axum::Router::new()
            .nest("/hello", hello::routes())
            // TODO this middleware layer will require the API key for all previous routes,
            // or alternatively you can use the `ApiKey` extractor in specific routes
            .layer(axum::middleware::from_extractor_with_state::<ApiKey, _>(
                state.clone(),
            ))
            .nest("/health", health::routes());

        Ok(router.nest("/api", api_routes))
   })
}
