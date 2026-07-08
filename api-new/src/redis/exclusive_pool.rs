use std::{sync::Arc, time::Duration};

use fred::interfaces::ClientLike;

/// The dynamic pool of Redis clients with exclusive connections for streaming / long-running tasks.
/// Stored in axum state.
pub type ExclusiveClientPool = deadpool::managed::Pool<ExclusiveClientManager>;
/// The error when failing to get a client from the exclusive pool
pub type ExclusiveClientPoolError = deadpool::managed::PoolError<fred::error::Error>;

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
            .recycle_timeout(Some(timeout))
            .wait_timeout(Some(timeout))
            .build()
    }

    pub async fn shutdown(&self) {
        for client in self.clients.lock().await.iter() {
            if let Err(err) = client.quit().await {
                tracing::warn!("Failed to shutdown Redis exclusive client: {err}");
            }
        }
    }

    pub async fn cleanup_task(
        exclusive_pool: ExclusiveClientPool,
        interval: Duration,
        idle_time: Duration,
    ) {
        let mut interval = tokio::time::interval(interval);
        loop {
            interval.tick().await;
            exclusive_pool.retain(|_, metrics| metrics.last_used() < idle_time);
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
        let _: () = client.ping(None).await?;

        Ok(())
    }

    fn detach(&self, client: &mut Self::Type) {
        let client = client.clone();
        let clients = self.clients.clone();

        tokio::spawn(async move {
            {
                clients.lock().await.retain(|c| c.id() != client.id());
            }
            if let Err(err) = client.quit().await {
                tracing::warn!("error disconnecting Redis client: {err}");
            }
        });
    }
}
