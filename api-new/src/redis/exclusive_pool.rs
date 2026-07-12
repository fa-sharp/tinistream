use std::{sync::Arc, time::Duration};

use fred::interfaces::ClientLike;

/// The dynamic pool of Redis clients with exclusive connections for streaming / long-running tasks.
/// Stored in axum state.
pub type ExclusiveClientPool = deadpool::managed::Pool<ExclusiveClientManager>;
/// The error when failing to get a client from the exclusive pool
pub type ExclusiveClientPoolError = deadpool::managed::PoolError<fred::error::Error>;

/// Interval to check for and clean up idle clients.
const IDLE_TASK_INTERVAL: Duration = Duration::from_secs(10);
/// Shut down clients after this period of inactivity.
const IDLE_TIME: Duration = Duration::from_secs(30);

/// Deadpool implementation for a dynamic pool of exclusive Redis clients.
#[derive(Debug)]
pub struct ExclusiveClientManager {
    client_config: fred::clients::Client,
    clients: Arc<tokio::sync::Mutex<Vec<fred::clients::Client>>>,
}

impl ExclusiveClientManager {
    pub fn new(client_config: fred::clients::Client) -> Self {
        Self {
            client_config,
            clients: Arc::default(),
        }
    }

    pub fn build_dynamic_pool(
        self,
        max_clients: usize,
        timeout: Duration,
    ) -> Result<ExclusiveClientPool, deadpool::managed::BuildError> {
        ExclusiveClientPool::builder(self)
            .max_size(max_clients)
            .runtime(deadpool::Runtime::Tokio1)
            .create_timeout(Some(timeout))
            .wait_timeout(Some(timeout))
            .recycle_timeout(Some(timeout))
            .build()
    }

    pub async fn shutdown(&self) {
        futures::future::join_all(self.clients.lock().await.iter().map(|c| c.quit())).await;
    }

    pub async fn cleanup_task(exclusive_pool: ExclusiveClientPool) {
        let mut interval = tokio::time::interval(IDLE_TASK_INTERVAL);
        loop {
            interval.tick().await;
            exclusive_pool.retain(|_, metrics| metrics.last_used() < IDLE_TIME);
        }
    }
}

impl deadpool::managed::Manager for ExclusiveClientManager {
    type Type = fred::prelude::Client;
    type Error = fred::error::Error;

    async fn create(&self) -> Result<fred::prelude::Client, Self::Error> {
        let client = self.client_config.clone_new();
        client.init().await?;
        self.clients.lock().await.push(client.clone());

        Ok(client)
    }

    async fn recycle(
        &self,
        client: &mut fred::clients::Client,
        _: &deadpool::managed::Metrics,
    ) -> deadpool::managed::RecycleResult<Self::Error> {
        if !client.is_connected() {
            client.init().await?;
        }

        /// Ping options when recycling a Redis client - quickly ensures a working connection
        const PING_OPTIONS: &fred::prelude::Options = &fred::prelude::Options {
            timeout: Some(Duration::from_millis(100)), // TODO make this configurable
            fail_fast: true,
            max_attempts: Some(1),
            max_redirections: None,
            cluster_node: None,
            cluster_hash: None,
        };
        let _: () = client.with_options(PING_OPTIONS).ping(None).await?;

        Ok(())
    }

    fn detach(&self, client: &mut Self::Type) {
        let client = client.clone();
        let all_clients = Arc::clone(&self.clients);

        tokio::spawn(async move {
            {
                let mut all_clients_lock = all_clients.lock().await;
                all_clients_lock.retain(|c| c.id() != client.id());
            }

            if client.is_connected() {
                let _ = client.quit().await;
            }
        });
    }
}
