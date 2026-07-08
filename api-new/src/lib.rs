use axum_plugin::{App, InitializedApp};

use crate::{config::AppConfig, state::AppState};

mod config;
mod error;
mod extractors;
mod plugins;
mod routes;
mod state;

pub async fn create_app() -> anyhow::Result<InitializedApp<AppState, AppConfig>> {
    let app = App::from_env_and_file("config.toml", "STREAMER_")?
        .register(routes::plugin()) // Add API routes
        .register(plugins::logging::plugin()) // Request logging
        .register(plugins::security::plugin()) // Body limit, security headers, etc.
        .init()
        .await?;

    Ok(app)
}
