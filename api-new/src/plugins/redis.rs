use std::{sync::Arc, time::Duration};

use anyhow::Context;
use fred::{prelude::*, socket2::TcpKeepalive};

use crate::{
    plugins::Plugin,
    redis::{ExclusiveClientManager, RedisWriter},
};

pub fn plugin() -> Plugin {
    Plugin::named("Redis")
        .on_init(async |mut app| {
            let config = app.config();
            let redis_config = Config::from_url(&config.redis_url).context("parse Redis URL")?;
            let timeout = Duration::from_secs(config.redis_timeout.into());
            let static_pool = Builder::from_config(redis_config)
                .with_connection_config(|config| {
                    config.connection_timeout = timeout;
                    config.internal_command_timeout = timeout;
                    config.max_command_attempts = 2;
                    config.tcp.nodelay = Some(true);
                    config.tcp.keepalive =
                        Some(TcpKeepalive::new().with_time(Duration::from_secs(10)));
                })
                .with_performance_config(|config| {
                    config.default_command_timeout = timeout;
                })
                .set_policy(ReconnectPolicy::new_linear(5, 2_000, 500))
                .build_pool(config.redis_pool)?;
            static_pool.init().await.context("connect to Redis")?;

            let exclusive_clients = ExclusiveClientManager::new(
                static_pool.next().clone_new(),
                config.max_clients,
                config.redis_timeout,
            );

            let ingest_hash = RedisWriter::load_ingest_script(&static_pool).await?;

            app.insert(static_pool)?;
            app.insert(exclusive_clients)?;
            app.insert(Arc::<str>::from(ingest_hash))?;
            Ok(app)
        })
        .on_shutdown(async |app| {
            tracing::info!("Shutting down Redis connections...");
            let _ = app.state().static_pool.quit().await;
            app.state().exclusive_clients.shutdown().await;

            Ok(())
        })
}
