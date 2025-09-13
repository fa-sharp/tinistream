mod client;
pub use client::RedisClient;

mod reader;
pub use reader::RedisReader;

pub mod constants;
pub mod util;

use fred::prelude::{Builder, Client, ClientLike, Config, Pool, ReconnectPolicy, TcpConfig};
use rocket::fairing::AdHoc;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

use crate::config::get_app_config;

/// Default size of the static Redis pool.
const REDIS_POOL_SIZE: usize = 4;
/// Default maximum number of concurrent exclusive clients (e.g. max concurrent streams)
const MAX_EXCLUSIVE_CLIENTS: usize = 20;
/// Timeout for connecting and executing commands.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(6);
/// Interval to check for idle exclusive clients.
const IDLE_TASK_INTERVAL: Duration = Duration::from_secs(120);
/// Shut down exclusive clients after this period of inactivity.
const IDLE_TIME: Duration = Duration::from_secs(60 * 5);

/// Redis setup fairing
pub fn setup_redis() -> AdHoc {
    AdHoc::on_ignite("Redis", |rocket| async {
        rocket
            .attach(AdHoc::on_ignite("Initialize Redis pools", |rocket| async {
                let app_config = get_app_config(&rocket);
                let redis_config =
                    Config::from_url(&app_config.redis_url).expect("Invalid Redis URL");
                let static_pool = Builder::from_config(redis_config)
                    .with_connection_config(|config| {
                        config.connection_timeout = CLIENT_TIMEOUT;
                        config.internal_command_timeout = CLIENT_TIMEOUT;
                        config.max_command_attempts = 2;
                        config.tcp = TcpConfig {
                            nodelay: Some(true),
                            ..Default::default()
                        };
                    })
                    .set_policy(ReconnectPolicy::new_linear(0, 10_000, 1000))
                    .with_performance_config(|config| {
                        config.default_command_timeout = CLIENT_TIMEOUT;
                    })
                    .build_pool(app_config.redis_pool.unwrap_or(REDIS_POOL_SIZE))
                    .expect("Failed to build Redis pool");
                static_pool.init().await.expect("Redis connection failed");

                // Build and initialize the dynamic pool of exclusive clients for long-running tasks
                let exclusive_manager = ExclusiveClientManager::new(static_pool.clone());
                let exclusive_pool: ExclusiveClientPool =
                    deadpool::managed::Pool::builder(exclusive_manager)
                        .max_size(app_config.max_streams.unwrap_or(MAX_EXCLUSIVE_CLIENTS))
                        .runtime(deadpool::Runtime::Tokio1)
                        .create_timeout(Some(CLIENT_TIMEOUT))
                        .recycle_timeout(Some(CLIENT_TIMEOUT))
                        .wait_timeout(Some(CLIENT_TIMEOUT))
                        .build()
                        .expect("Failed to build exclusive Redis pool");

                // Spawn a task to periodically clean up idle exclusive clients
                let idle_task_pool = exclusive_pool.clone();
                tokio::spawn(async move {
                    let mut interval = tokio::time::interval(IDLE_TASK_INTERVAL);
                    loop {
                        interval.tick().await;
                        idle_task_pool.retain(|_, metrics| metrics.last_used() < IDLE_TIME);
                    }
                });

                rocket.manage(static_pool).manage(exclusive_pool)
            }))
            .attach(AdHoc::on_shutdown("Shutdown Redis pools", |rocket| {
                Box::pin(async {
                    if let Some(pool) = rocket.state::<Pool>() {
                        rocket::info!("Shutting down static Redis pool");
                        if let Err(err) = pool.quit().await {
                            rocket::warn!("Failed to shutdown static Redis pool: {}", err);
                        }
                    }
                    if let Some(exclusive_pool) = rocket.state::<ExclusiveClientPool>() {
                        rocket::info!("Shutting down exclusive Redis pool");
                        for client in exclusive_pool.manager().clients.lock().await.iter() {
                            if let Err(err) = client.quit().await {
                                rocket::warn!("Failed to shutdown Redis client: {}", err);
                            }
                        }
                    }
                })
            }))
    })
}

/// A pool of Redis clients with exclusive connections for long-running operations. Will
/// be stored in Rocket's managed state.
pub type ExclusiveClientPool = deadpool::managed::Pool<ExclusiveClientManager>;

/// Deadpool implementation for a pool of exclusive Redis clients.
#[derive(Debug)]
pub struct ExclusiveClientManager {
    pool: fred::clients::Pool,
    clients: Arc<Mutex<Vec<Client>>>,
}

impl ExclusiveClientManager {
    pub fn new(pool: fred::clients::Pool) -> Self {
        Self {
            pool,
            clients: Arc::default(),
        }
    }
}
impl deadpool::managed::Manager for ExclusiveClientManager {
    type Type = Client;
    type Error = fred::error::Error;

    async fn create(&self) -> Result<Client, Self::Error> {
        let client = self.pool.next().clone_new();
        client.init().await?;
        self.clients.lock().await.push(client.clone());
        Ok(client)
    }

    async fn recycle(
        &self,
        client: &mut Client,
        _: &deadpool::managed::Metrics,
    ) -> deadpool::managed::RecycleResult<Self::Error> {
        if !client.is_connected() {
            client.init().await?;
        }
        let _: () = client.ping(None).await?;
        Ok(())
    }

    fn detach(&self, client: &mut Self::Type) {
        let client = client.clone();
        let clients = self.clients.clone();
        tokio::spawn(async move {
            clients.lock().await.retain(|c| c.id() != client.id());
            if let Err(err) = client.quit().await {
                rocket::error!("Failed to disconnect Redis client: {}", err);
            }
        });
    }
}
