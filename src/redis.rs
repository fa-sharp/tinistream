mod client;
pub use client::RedisClient;

pub mod constants;
mod util;

use fred::prelude::{Builder, ClientLike, Config, Pool, TcpConfig};
use rocket::fairing::AdHoc;
use std::time::Duration;

use crate::config::get_app_config;

/// Redis setup fairing
pub fn setup_redis() -> AdHoc {
    AdHoc::on_ignite("Redis", |rocket| async {
        rocket
            .attach(AdHoc::on_ignite(
                "Initialize Redis connection",
                |rocket| async {
                    let app_config = get_app_config(&rocket);
                    let redis_config =
                        Config::from_url(&app_config.redis_url).expect("Invalid Redis URL");
                    let pool = Builder::from_config(redis_config)
                        .with_connection_config(|config| {
                            config.connection_timeout = Duration::from_secs(4);
                            config.tcp = TcpConfig {
                                nodelay: Some(true),
                                ..Default::default()
                            };
                        })
                        .build_pool(app_config.redis_pool.unwrap_or(2))
                        .expect("Failed to build Redis pool");

                    pool.init().await.expect("Failed to connect to Redis");
                    tracing::info!("Redis connection initialized");

                    rocket.manage(pool)
                },
            ))
            .attach(AdHoc::on_shutdown("Shutdown Redis connection", |rocket| {
                Box::pin(async {
                    if let Some(pool) = rocket.state::<Pool>() {
                        tracing::info!("Shutting down Redis connection");
                        if let Err(err) = pool.quit().await {
                            tracing::error!("Failed to shutdown Redis: {}", err);
                        }
                    }
                })
            }))
    })
}
