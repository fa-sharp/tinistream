use std::net::{IpAddr, Ipv4Addr};

use anyhow::Context;
use axum_plugin::AdHocPlugin;
use serde::Deserialize;

use crate::state::AppState;

/// Parsed app configuration
#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    // Server config
    pub host: IpAddr,
    pub port: u16,
    pub log_level: String,
    pub request_id_header: String,

    // Auth
    pub api_key: String,
    pub api_key_header: String,

    // Security
    pub allowed_origins: Option<String>,
    pub body_limit: usize,
}
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 8080,
            log_level: "info".into(),
            request_id_header: "x-request-id".into(),
            api_key: "api-key-123".into(),
            api_key_header: "x-api-key".into(),
            allowed_origins: None,
            body_limit: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// Plugin that reads and validates configuration, and adds it to server state
pub fn plugin() -> AdHocPlugin<AppState> {
    AdHocPlugin::named("Config").on_init(async |mut state| {
        let config = extract_config()?;
        state.insert(config);
        Ok(state)
    })
}

/// Extract the configuration from env variables prefixed with `STREAMER_`.
fn extract_config() -> anyhow::Result<AppConfig> {
    let config = figment::Figment::new()
        .merge(figment::providers::Env::prefixed("STREAMER_"))
        .extract::<AppConfig>()
        .context("Failed to extract valid configuration")?;

    Ok(config)
}
