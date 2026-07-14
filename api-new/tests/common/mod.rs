use std::time::Duration;

use axum_test::TestServer;
use tinistream_api::create_app;

/// Setup the server listening on a random port. Returns the port, server, and shutdown future
pub async fn setup_http_server() -> anyhow::Result<(
    u16,
    TestServer,
    impl Future<Output = anyhow::Result<()>> + Send,
)> {
    dotenvy::dotenv().ok();
    let app = create_app().await?;
    let server = TestServer::builder().http_transport().build(app.router());
    let port = server.server_address().unwrap().port().unwrap();

    Ok((port, server, app.shutdown()))
}

/// Setup the tinistream Rust client with a backend API key
pub fn setup_backend_client(port: u16) -> tinistream_client::Client {
    use reqwest::header::HeaderMap;

    let api_key = dotenvy::var("STREAMER_API_KEY").expect("API key not set");
    let mut api_key_header = HeaderMap::new();
    api_key_header.insert("X-API-KEY", api_key.parse().unwrap());

    let http_client = reqwest::Client::builder()
        .default_headers(api_key_header)
        .build()
        .expect("build client");
    let client = tinistream_client::Client::new_with_client(
        &format!("http://localhost:{port}"),
        http_client,
    );

    client
}

/// Setup reqwest client for frontend API requests
pub fn setup_frontend_client(token: &str) -> reqwest::Client {
    let mut token_header = reqwest::header::HeaderMap::new();
    token_header.insert("Authorization", format!("Bearer {token}").parse().unwrap());
    let http_client = reqwest::Client::builder()
        .default_headers(token_header)
        .build()
        .expect("build client");

    http_client
}

/// Spawn task to add 10 `(test_event, test_data)` events to the Redis stream on an interval
pub fn add_events_task(
    client: tinistream_client::Client,
    key: &str,
) -> tokio::task::JoinHandle<()> {
    use tinistream_client::types::{AddEvent, AddEventsRequest, StreamRequest};
    use tinistream_client::{ClientIngestExt, ClientStreamExt};

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
            if let Err(e) = client.add_events().body(body).send().await {
                eprintln!("Error in add events task: {e}");
            }
        }
        let _ = client
            .end_stream()
            .body(StreamRequest::builder().key(key))
            .send()
            .await;
    })
}
