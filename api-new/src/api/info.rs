use axum::{Json, extract::State, routing};
use schemars::JsonSchema;
use serde::Serialize;

use crate::state::AppState;

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/", routing::get(get_info))
}

/// Get information about the server
async fn get_info<'r>(State(state): State<AppState>) -> Json<InfoResponse> {
    let redis_status = state.exclusive_pool.status();
    let redis_stats = RedisStats {
        r#static: state.config.redis_pool,
        streaming: redis_status.size,
        streaming_in_use: redis_status.size - redis_status.available,
        streaming_available: redis_status.available,
        streaming_max: redis_status.max_size,
    };

    Json(InfoResponse {
        url: state.config.base_url.clone(),
        version: format!("v{}", env!("CARGO_PKG_VERSION")),
        redis: redis_stats,
    })
}

#[derive(Debug, Serialize, JsonSchema)]
struct InfoResponse {
    url: String,
    version: String,
    redis: RedisStats,
}

#[derive(Debug, Serialize, JsonSchema)]
struct RedisStats {
    /// Number of static connections
    r#static: usize,
    /// Number of streaming connections
    streaming: usize,
    /// Number of in-use streaming connections
    streaming_in_use: usize,
    /// Number of available streaming connections
    streaming_available: usize,
    /// Maximum number of streaming connections
    streaming_max: usize,
}
