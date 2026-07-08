use std::time::Duration;

use anyhow::Context;
use fred::prelude::*;

/// Static pool of Redis clients stored in axum state
pub type StaticPool = Pool;

pub struct RedisClient;

impl RedisClient {
    pub async fn connect(
        redis_url: &str,
        pool_size: usize,
        client_timeout: Duration,
    ) -> anyhow::Result<StaticPool> {
        let redis_config = Config::from_url(redis_url).context("Invalid Redis URL")?;
        let pool = Builder::from_config(redis_config)
            .with_connection_config(|config| {
                config.connection_timeout = client_timeout;
                config.internal_command_timeout = client_timeout;
                config.max_command_attempts = 2;
                config.tcp = TcpConfig {
                    nodelay: Some(true),
                    ..Default::default()
                };
            })
            .set_policy(ReconnectPolicy::new_linear(0, 10_000, 1000))
            .with_performance_config(|config| {
                config.default_command_timeout = client_timeout;
            })
            .build_pool(pool_size)
            .context("Failed to build Redis pool")?;
        pool.init().await.context("Redis connection failed")?;

        Ok(pool)
    }
}
