# tinistreamer-client

A Rust client for the tinistreamer API.

## Generate

To generate or update the client, run the following commands in the project root:

```bash
# Update the OpenAPI spec (make sure the server is running locally)
curl -o spec/openapi.json http://localhost:8000/api/openapi.json

# Install cargo-progenitor
cargo install cargo-progenitor

# Generate the client
cargo progenitor -i openapi.json -o clients/rust/ -n tinistreamer-client --interface builder --tags separate --version 0.1.0 --license-name MIT
```
