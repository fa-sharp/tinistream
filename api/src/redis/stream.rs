use std::sync::Arc;

use crate::{config::AppConfig, redis::constants};

/// Utilities for managing Redis streams
pub struct StreamService {
    config: Arc<AppConfig>,
}

impl StreamService {
    pub fn new(config: Arc<AppConfig>) -> Self {
        Self { config }
    }

    /// Get the full stream key/prefix
    pub fn stream_key(&self, key: &str) -> String {
        [&self.config.key_prefix, constants::STREAM_PREFIX, key].concat()
    }

    /// Get the full key for the metadata associated with a given stream key/prefix
    pub fn meta_key(&self, key: &str) -> String {
        [&self.config.key_prefix, constants::META_PREFIX, key].concat()
    }

    /// Length of the meta key prefix
    pub fn meta_key_prefix_len(&self) -> usize {
        self.config.key_prefix.len() + constants::META_PREFIX.len()
    }

    /// Get the URL for streaming SSE events from the given Redis stream
    pub fn sse_url(&self, key: &str) -> String {
        format!(
            "{}/api/client/sse?key={}",
            self.config.base_url,
            urlencoding::encode(key)
        )
    }

    /// Get the URL for streaming WebSocket events from the given Redis stream
    pub fn ws_url(&self, key: &str) -> String {
        format!(
            "{}/api/client/ws?key={}",
            &self.config.base_url,
            urlencoding::encode(key)
        )
    }
}
