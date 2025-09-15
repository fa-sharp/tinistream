use rocket::{get, serde::json::Json, Route, State};
use rocket_okapi::{okapi::openapi3::OpenApi, openapi, openapi_get_routes_spec};
use schemars::JsonSchema;
use serde::Serialize;

use crate::{auth::ApiKeyAuth, config::AppConfig, redis::ExclusiveClientPool};

pub fn get_routes() -> (Vec<Route>, OpenApi) {
    openapi_get_routes_spec![health, get_info]
}

/// # Health check
#[openapi(tag = "Info")]
#[get("/health")]
fn health() -> &'static str {
    "OK"
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
    /// Number of current streaming connections
    streaming: usize,
    /// Number of available streaming connections
    streaming_available: usize,
    /// Maximum number of streaming connections
    streaming_max: usize,
}

/// # Get info
/// Get information about the server
#[openapi(tag = "Info")]
#[get("/info")]
async fn get_info(
    _api_key: ApiKeyAuth,
    app_config: &State<AppConfig>,
    redis_pool: &State<ExclusiveClientPool>,
) -> Json<InfoResponse> {
    let redis_status = redis_pool.status();
    let redis_stats = RedisStats {
        r#static: app_config.redis_pool.unwrap_or(4),
        streaming: redis_status.size,
        streaming_max: redis_status.max_size,
        streaming_available: redis_status.available,
    };

    Json(InfoResponse {
        url: app_config.server_address.clone(),
        version: format!("v{}", env!("CARGO_PKG_VERSION")),
        redis: redis_stats,
    })
}
