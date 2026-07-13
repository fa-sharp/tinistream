use std::{
    ops::Deref,
    sync::{Arc, Mutex},
    time::Duration,
};

use fred::{
    clients::Client,
    interfaces::{ClientLike, FredResult},
    types::ConnectHandle,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

/// Keeps track of the currently checked-out exclusive clients
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
    #[allow(unused)]
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

        futures::future::join_all(clients.into_iter().map(async |(client, handle)| {
            if let Ok(_) = client.quit().await {
                let _ = handle.await;
            }
        }))
        .await;
    }
}

impl Drop for ExclusiveClient {
    fn drop(&mut self) {
        let client = self.client.clone();
        let clients = Arc::clone(&self.clients);
        let permit = self.permit.take();

        tokio::spawn(async move {
            if let Some((client, handle)) = {
                let mut clients_lock = clients.lock().unwrap();
                clients_lock
                    .extract_if(.., |(c, _)| c.id() == client.id())
                    .next()
            } {
                if let Ok(_) = client.quit().await {
                    let _ = handle.await;
                }
            }

            drop(permit);
        });
    }
}
