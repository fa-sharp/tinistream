# Clients

Before generating clients, make sure to update the OpenAPI spec to ensure it is in sync with any changes made to the server. While the dev server is running, run the following command to update the spec:

```bash
curl -o spec/openapi.json http://localhost:8000/openapi.json
```

## Python

### Generate

Run the following commands in the project root:

```bash
# Create a virtual environment and install openapi-python-client
python3 -m venv .venv
source .venv/bin/activate
pip install openapi-python-client

# Generate the client
openapi-python-client generate --path spec/openapi.json --output-path clients/python --config clients/python/config.yml --custom-template-path clients/python/templates --overwrite
```

## Rust

### Generate

Run the following commands in the project root:

```bash
# If needed, install cargo-progenitor
cargo install cargo-progenitor

# Generate the client
cargo progenitor -i spec/openapi.json -o clients/rust/ -n tinistream-client --interface builder --tags separate --license-name MIT --version 0.1.0
```
