use std::{
    ops::Deref,
    sync::{Arc, Mutex},
    time::Duration,
};

use fred::{
    clients::Client,
    interfaces::{ClientInterface, ClientLike, FredResult},
    types::ConnectHandle,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Keeps track of the currently checked-out exclusive clients and connections
type CurrentClients = Arc<Mutex<Vec<(Client, ConnectHandle)>>>;

/// Manager that hands out clients with exclusive Redis connections for long-running commands
pub struct ExclusiveClientManager {
    semaphore: Arc<Semaphore>,
    client_config: Client,
    clients: CurrentClients,
    wait_timeout_secs: u64,
}

/// A Redis client with a permit for an exclusive connection
pub struct ExclusiveClient {
    client: Client,
    clients: CurrentClients,
    permit: Option<OwnedSemaphorePermit>,
}
impl Deref for ExclusiveClient {
    type Target = Client;
    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl ExclusiveClientManager {
    pub fn new(client_config: Client, max_clients: usize, wait_timeout_secs: u32) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_clients)),
            client_config,
            clients: Mutex::new(Vec::with_capacity(10)).into(),
            wait_timeout_secs: wait_timeout_secs.into(),
        }
    }

    /// Get an initialized/connected Redis client with a permit for an exclusive connection.
    /// Will return `None` if there are too many connections.
    pub async fn get(&self) -> FredResult<Option<ExclusiveClient>> {
        let permit = match Arc::clone(&self.semaphore).try_acquire_owned() {
            Ok(permit) => permit,
            Err(_) => match tokio::time::timeout(
                Duration::from_secs(self.wait_timeout_secs),
                Arc::clone(&self.semaphore).acquire_owned(),
            )
            .await
            {
                Ok(Ok(permit)) => permit,
                _ => return Ok(None),
            },
        };

        let client = self.client_config.clone_new();
        let handle = client.init().await?;
        self.clients.lock().unwrap().push((client.clone(), handle));

        Ok(Some(ExclusiveClient {
            client,
            permit: Some(permit),
            clients: Arc::clone(&self.clients),
        }))
    }

    /// Number of currently available exclusive connections
    pub fn num_available(&self) -> usize {
        self.semaphore.available_permits()
    }

    /// Prevent checking out more exclusive clients, and shut down all checked-out clients
    pub async fn shutdown(&self) {
        self.semaphore.close();

        let clients: Vec<_> = {
            let mut clients_lock = self.clients.lock().unwrap();
            clients_lock.drain(..).collect()
        };

        futures::future::join_all(clients.into_iter().map(shutdown_client)).await;
    }
}

/// Shutdown the exclusive client, unblocking any blocking command
/// and shutting down the connection handle with a grace period
async fn shutdown_client((client, handle): (Client, ConnectHandle)) {
    let _ = client.unblock_self(None).await;
    if let Err(e) = client.quit().await {
        tracing::warn!("Failed to quit Redis client {}: {e}", client.id());
    }
    let abort_handle = handle.abort_handle();
    if let Err(_) = tokio::time::timeout(Duration::from_secs(5), handle).await {
        tracing::warn!("Aborting connection for Redis client {}...", client.id());
        abort_handle.abort();
    }
}

impl Drop for ExclusiveClient {
    fn drop(&mut self) {
        let client = self.client.clone();
        let clients = Arc::clone(&self.clients);
        let permit = self.permit.take();

        tokio::spawn(async move {
            if let Some(client) = {
                let mut clients_lock = clients.lock().unwrap();
                clients_lock
                    .extract_if(.., |(c, _)| c.id() == client.id())
                    .next()
            } {
                shutdown_client(client).await;
            }

            drop(permit);
        });
    }
}
