use std::net::{IpAddr, Ipv4Addr};

use serde::{Deserialize, Serialize};

/// Parsed app configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
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
