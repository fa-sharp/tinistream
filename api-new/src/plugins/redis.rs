use std::time::Duration;

use fred::interfaces::ClientLike;

use crate::{
    plugins::Plugin,
    redis::{ExclusiveClientManager, ExclusiveClientPool, RedisClient, StaticPool},
};

/// Timeout for Redis connections and commands (excluding long-running commands)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(6);
/// Interval to check for and clean up idle exclusive clients.
const IDLE_TASK_INTERVAL: Duration = Duration::from_secs(120);
/// Shut down exclusive clients after this period of inactivity.
const IDLE_TIME: Duration = Duration::from_secs(60 * 5);

pub fn plugin() -> Plugin {
    Plugin::named("Redis")
        .on_init(async |mut app| {
            let config = app.config();
            let static_pool: StaticPool =
                RedisClient::connect(&config.redis_url, config.redis_pool, DEFAULT_TIMEOUT).await?;

            let exclusive_manager = ExclusiveClientManager::new(static_pool.next().clone_new());
            let exclusive_pool: ExclusiveClientPool =
                exclusive_manager.build_dynamic_pool(config.max_clients, DEFAULT_TIMEOUT)?;

            tokio::spawn(ExclusiveClientManager::cleanup_task(
                exclusive_pool.clone(),
                IDLE_TASK_INTERVAL,
                IDLE_TIME,
            ));

            app.insert(static_pool)?;
            app.insert(exclusive_pool)?;
            Ok(app)
        })
        .on_shutdown(|app| {
            let static_pool = app.state().static_pool.clone();
            let exclusive_pool = app.state().exclusive_pool.clone();

            async move {
                tracing::info!("Shutting down Redis pools");
                if let Err(err) = static_pool.quit().await {
                    tracing::warn!("Failed to shutdown Redis static pool: {err}");
                }
                exclusive_pool.manager().shutdown().await;

                Ok(())
            }
        })
}
