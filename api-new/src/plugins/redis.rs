use std::time::Duration;

use anyhow::Context;
use fred::prelude::*;

use crate::{
    plugins::Plugin,
    redis::{ExclusiveClientManager, ExclusiveClientPool},
};

/// Interval to check for and clean up idle exclusive clients.
const IDLE_TASK_INTERVAL: Duration = Duration::from_secs(120);
/// Shut down exclusive clients after this period of inactivity.
const IDLE_TIME: Duration = Duration::from_secs(60 * 5);

pub fn plugin() -> Plugin {
    Plugin::named("Redis")
        .on_init(async |mut app| {
            let config = app.config();
            let redis_config = Config::from_url(&config.redis_url).context("invalid Redis URL")?;
            let connect_timeout = Duration::from_secs(config.redis_timeout.into());
            let static_pool = Builder::from_config(redis_config)
                .with_connection_config(|config| {
                    config.connection_timeout = connect_timeout;
                    config.internal_command_timeout = connect_timeout;
                    config.max_command_attempts = 2;
                })
                .set_policy(ReconnectPolicy::new_linear(0, 10_000, 1000))
                .build_pool(config.redis_pool)?;
            static_pool
                .init()
                .await
                .context("failed to connect to Redis")?;

            let exclusive_manager = ExclusiveClientManager::new(static_pool.next().clone_new());
            let exclusive_pool: ExclusiveClientPool =
                exclusive_manager.build_dynamic_pool(config.max_clients, connect_timeout)?;

            tokio::spawn(ExclusiveClientManager::cleanup_task(
                exclusive_pool.clone(),
                IDLE_TASK_INTERVAL,
                IDLE_TIME,
            ));

            app.insert(static_pool)?;
            app.insert(exclusive_pool)?;
            Ok(app)
        })
        .on_shutdown(async |app| {
            tracing::info!("Shutting down Redis pools");
            if let Err(err) = app.state().static_pool.quit().await {
                tracing::warn!("Failed to shutdown Redis static pool: {err}");
            }
            app.state().exclusive_pool.manager().shutdown().await;

            Ok(())
        })
}
