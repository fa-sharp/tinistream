# tinistream-client

A Rust client for the tinistream API.

## Usage

To use the client, add the following to your `Cargo.toml`:

```toml
[dependencies]
tinistream-client = { git = "https://github.com/fa-sharp/tinistream.git" }
```

### Backend client

```rust
use tinistream_client::{types::*, ClientClientExt, ClientStreamExt};

let api_key = std::env::var("TINISTREAM_API_KEY").expect("API key not set");
let mut api_key_header = reqwest::header::HeaderMap::new();
api_key_header.insert("X-API-KEY", api_key.parse().unwrap());

let http_client = reqwest::Client::builder()
    .default_headers(api_key_header)
    .build()
    .expect("build client");
let client = Client::new_with_client("http://localhost:8000", http_client);

let stream_key = "my_stream";
let res = backend_client
    .create_stream()
    .body(StreamRequest::builder().key(&stream_key))
    .send()
    .await;
```

## Development

### Generate

To generate or update the client from the OpenAPI spec, run the following commands in the project root:

```bash
# Update the OpenAPI spec (make sure the server is running locally)
curl -o spec/openapi.json http://localhost:8000/openapi.json

# If needed, install cargo-progenitor
cargo install cargo-progenitor

# Generate the client
cargo progenitor -i spec/openapi.json -o clients/rust/ -n tinistream-client --interface builder --tags separate --license-name MIT --version 0.1.0
```
