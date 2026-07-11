use std::time::Duration;

use anyhow::Context;
use fred::prelude::*;

use crate::{
    plugins::Plugin,
    redis::{ExclusiveClientManager, ExclusiveClientPool},
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
                })
                .with_performance_config(|config| {
                    config.default_command_timeout = timeout;
                })
                .set_policy(ReconnectPolicy::new_linear(5, 5_000, 500))
                .build_pool(config.redis_pool)?;
            static_pool.init().await.context("connect to Redis")?;

            let exclusive_manager = ExclusiveClientManager::new(static_pool.next().clone_new());
            let exclusive_pool: ExclusiveClientPool =
                exclusive_manager.build_dynamic_pool(config.max_clients, timeout)?;

            tokio::spawn(ExclusiveClientManager::cleanup_task(exclusive_pool.clone()));

            app.insert(static_pool)?;
            app.insert(exclusive_pool)?;
            Ok(app)
        })
        .on_shutdown(async |app| {
            tracing::info!("Shutting down Redis pools...");
            let _ = app.state().static_pool.quit().await;
            app.state().exclusive_pool.manager().shutdown().await;

            Ok(())
        })
}
