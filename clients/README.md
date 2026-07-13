# Clients

Before generating clients, make sure to update the OpenAPI spec to ensure it is in sync with any changes made to the server. While the dev server is running, run the following command to update the spec:

```bash
curl -o spec/openapi.json http://localhost:8000/api/docs/openapi.json
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
# Install cargo-progenitor (using fork until OpenAPI v3.1 is supported: https://github.com/oxidecomputer/progenitor/issues/1268)
cargo install cargo-progenitor --git https://github.com/WalletConnect/progenitor --rev 604aacb0df

# Generate the client
cargo progenitor -i spec/openapi.json -o clients/rust/ -n tinistream-client --interface builder --tags separate --license-name MIT --version <version>
```
