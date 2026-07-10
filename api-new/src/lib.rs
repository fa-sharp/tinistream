use axum_plugin::{App, InitializedApp};

use crate::{config::AppConfig, state::AppState};

mod api;
mod auth;
mod config;
mod error;
mod extractors;
mod plugins;
mod redis;
mod state;

pub async fn create_app() -> anyhow::Result<InitializedApp<AppState, AppConfig>> {
    let app = App::from_env_and_file("STREAMER_", "config.toml")?
        .register(plugins::crypto::plugin()) // Add token encryption
        .register(plugins::redis::plugin()) // Connect and setup Redis pools
        .register(api::plugin()) // Add API routes
        .register(plugins::logging::plugin()) // Request logging
        .register(plugins::security::plugin()) // Body limit, security headers, etc.
        .init()
        .await?;

    Ok(app)
}
