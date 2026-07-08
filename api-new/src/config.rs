use std::net::{IpAddr, Ipv4Addr};

use serde::{Deserialize, Serialize};

/// Parsed app configuration
#[derive(Debug, Serialize, Deserialize)]
pub struct AppConfig {
    // Server config
    pub host: IpAddr,
    pub port: u16,
    pub base_url: String,
    pub log_level: String,
    pub request_id_header: String,

    // Auth
    pub api_key: String,
    pub api_key_header: String,
    /// 32-byte hex string (64 characters) used for encrypting client tokens
    pub secret_key: String,

    // Redis
    pub redis_url: String,
    /// Redis static pool size (default: 4)
    pub redis_pool: usize,
    /// Default TTL in seconds for Redis streams (default: 10 minutes)
    pub redis_ttl: u32,
    /// Maximum number of concurrent reading clients (default: 50)
    pub max_clients: usize,

    // Security
    /// Allowed origins for CORS, comma-separated list of domains (all domains allowed by default)
    pub allowed_origins: Option<String>,
    pub body_limit: usize,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            host: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 8081,
            base_url: "http://localhost:8081".into(),
            log_level: "info".into(),
            request_id_header: "x-request-id".into(),
            api_key: String::new(),
            api_key_header: "x-api-key".into(),
            secret_key: String::new(),
            redis_url: "redis://localhost".into(),
            redis_pool: 4,
            redis_ttl: 600,
            max_clients: 50,
            allowed_origins: None,
            body_limit: 10 * 1024 * 1024, // 10 MB
        }
    }
}
