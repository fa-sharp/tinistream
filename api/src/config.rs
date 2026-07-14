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
    /// Timeout in seconds for Redis connections and commands (default: 4 seconds)
    pub redis_timeout: u32,
    /// Default TTL in seconds for Redis streams (default: 10 minutes)
    pub stream_ttl: u32,
    /// Prefix for all streams in Redis (default: "tinistream:")
    pub key_prefix: String,
    /// Timeout in seconds for client connections if there's no activity in the Redis stream (default: 5 minutes)
    pub client_timeout: u32,
    /// Maximum number of events in a Redis stream (default: 5000)
    pub max_stream_len: u32,
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
            port: 8000,
            base_url: "http://localhost:8000".into(),
            log_level: "info".into(),
            request_id_header: "x-request-id".into(),
            api_key: String::new(),
            api_key_header: "x-api-key".into(),
            secret_key: String::new(),
            redis_url: "redis://localhost".into(),
            redis_pool: 4,
            redis_timeout: 4,
            stream_ttl: 10 * 60,
            key_prefix: "tinistream:".into(),
            client_timeout: 5 * 60,
            max_stream_len: 5000,
            max_clients: 50,
            allowed_origins: None,
            body_limit: 10 * 1024 * 1024, // 10 MB
        }
    }
}
