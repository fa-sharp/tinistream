use std::time::Duration;

use reqwest::header::HeaderMap;
use rocket::{Ignite, Rocket};
use tinistream::build_rocket;
use tinistream_client::{Client, ClientStreamExt};
use tokio::{net::TcpListener, task::JoinHandle};

/// Run the Rocket server on a random port and return the handle, port, and shutdown signal.
pub async fn setup_rocket() -> Result<
    (
        JoinHandle<Result<Rocket<Ignite>, rocket::Error>>,
        u16,
        rocket::Shutdown,
    ),
    std::io::Error,
> {
    let rocket = build_rocket();
    let port = { TcpListener::bind("127.0.0.1:0").await?.local_addr()?.port() };
    let figment = rocket.figment().clone().merge((rocket::Config::PORT, port));

    let rocket = rocket.configure(figment).ignite().await.expect("ignite");
    let shutdown = rocket.shutdown();
    let handle = tokio::spawn(rocket.launch());

    Ok((handle, port, shutdown))
}

/// Setup the tinistream Rust client with a backend API key
pub fn setup_backend_client(port: u16) -> Client {
    use reqwest::header::HeaderMap;

    let api_key = dotenvy::var("STREAMER_API_KEY").expect("API key not set");
    let mut api_key_header = HeaderMap::new();
    api_key_header.insert("X-API-KEY", api_key.parse().unwrap());

    let http_client = reqwest::Client::builder()
        .default_headers(api_key_header)
        .build()
        .expect("build client");
    let client = Client::new_with_client(&format!("http://localhost:{port}"), http_client);

    client
}

/// Setup reqwest client for frontend API requests
pub fn setup_frontend_reqwest(token: &str) -> reqwest::Client {
    let mut token_header = HeaderMap::new();
    token_header.insert("Authorization", format!("Bearer {token}").parse().unwrap());
    let http_client = reqwest::Client::builder()
        .default_headers(token_header)
        .build()
        .expect("build client");
    http_client
}

/// Setup the tinistream Rust client with a frontend token
pub fn setup_frontend_client(port: u16, token: &str) -> Client {
    let http_client = setup_frontend_reqwest(token);
    let client = Client::new_with_client(&format!("http://localhost:{port}"), http_client);
    client
}

/// Spawn task to add 10 `(test_event, test_data)` events to the Redis stream on an interval
pub fn add_events_task(client: Client, key: &str) -> tokio::task::JoinHandle<()> {
    use tinistream_client::types::*;

    let key = key.to_owned();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        for _ in 0..10 {
            interval.tick().await;
            let test_event = AddEvent::builder()
                .data("test_data".to_owned())
                .event("test_event");
            let body = AddEventsRequest::builder()
                .key(&key)
                .events(vec![test_event.try_into().unwrap()]);
            let _ = client.add_events().body(body).send().await;
        }
        let _ = client
            .end_stream()
            .body(StreamRequest::builder().key(key))
            .send()
            .await;
    })
}
