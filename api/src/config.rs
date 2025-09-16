use rocket::{
    figment::{
        providers::{Env, Format, Toml},
        Figment,
    },
    Build, Rocket,
};
use serde::{Deserialize, Serialize};

/// Main server configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    /// Server address for URLs and redirects
    pub server_address: String,
    /// Redis connection URL
    pub redis_url: String,
    /// Redis static pool size (default: 4)
    pub redis_pool: Option<usize>,
    /// Maximum number of concurrent reading clients (default: 20)
    pub max_clients: Option<usize>,
    /// API key for creating and writing Redis streams
    pub api_key: String,
    /// 32-byte hex string (64 characters) used for encrypting client tokens
    pub secret_key: String,
    /// Default TTL in seconds for Redis streams (default: 10 minutes)
    #[serde(default = "default_stream_ttl")]
    pub ttl: u32,
}
fn default_stream_ttl() -> u32 {
    600
}

/// Get the server configuration from Rocket state
pub fn get_app_config(rocket: &Rocket<Build>) -> &AppConfig {
    rocket
        .state::<AppConfig>()
        .expect("Configuration not found!")
}

/// Build configuration provider from multiple sources
pub fn get_config_provider() -> Figment {
    #[cfg(debug_assertions)]
    if let Err(e) = dotenvy::dotenv() {
        println!("Failed to read .env file: {}", e);
    }

    Figment::from(rocket::Config::default())
        .merge(Toml::file("Rocket.toml").nested())
        .merge(Env::prefixed("STREAMER_").global())
}
