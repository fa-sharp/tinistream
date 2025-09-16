use rocket::{Ignite, Rocket};
use tinistream::build_rocket;
use tinistream_client::Client;
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

/// Setup the tinistream Rust client with a frontend token
pub fn setup_frontend_client(port: u16, token: &str) -> Client {
    use reqwest::header::HeaderMap;

    let mut token_header = HeaderMap::new();
    token_header.insert("Authorization", format!("Bearer {token}").parse().unwrap());

    let http_client = reqwest::Client::builder()
        .default_headers(token_header)
        .build()
        .expect("build client");
    let client = Client::new_with_client(&format!("http://localhost:{port}"), http_client);

    client
}
