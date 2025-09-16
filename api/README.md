# tinistream

A streaming microservice built with Rust, powered by Redis streams.

## Getting Started

### Prerequisites

- Rust 1.70+
- Redis

### Setup

1. **Clone and navigate to the project:**
   ```bash
   cd api
   ```

2. **Environment setup:**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

4. **Run the application:**
   ```bash
   cargo run
   ```

The server will start at `http://localhost:8000`


## Development

### Project Structure

```
src/
├── /api            # API route handlers
├── /auth           # Authentication and authorization
├── config.rs       # Defining and loading config variables
├── crypto.rs       # Encryption and decryption utilities
├── errors.rs       # Error types and handling
├── lib.rs          # Building the Rocket server and mounting routes
├── main.rs         # Application entry point
├── openapi.rs      # Building the OpenAPI spec
└── redis.rs        # Redis setup and utilities
```

## Deployment

### Docker

Build and run with Docker:
```bash
docker build -t tinistream .
docker run -p 8000:8000 tinistream
```

### Configuration

Configuration is handled through environment variables with the prefix `STREAMER_`:
- `STREAMER_REDIS_URL` - Redis connection string
- `STREAMER_REDIS_POOL` - Redis connection pool size
- `STREAMER_SECRET_KEY` - Secret key for encryption
- ..etc..

See [src/config.rs](src/config.rs) for all configuration options.
