# rs-stream

A Rocket web application generated with rocket-cli.

## Features

- 🚀 **Rocket** - Fast, secure, and flexible web framework
- 📦 **Redis** - High-performance caching and sessions
- 🔐 **Authentication** - Secure user authentication system
- 📝 **Structured Logging** - Comprehensive application logging
- ⚡ **Async/Await** - Modern asynchronous Rust

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
├── main.rs          # Application entry point
├── lib.rs           # Application builder and routing
├── config.rs        # Configuration management
├── errors.rs        # Error handling
├── redis.rs        # Redis connection and setup
├── auth/           # Authentication modules
└── api/            # API route handlers
```

### Adding New Routes

Generate a new API route:
```bash
rocket-cli generate route users --crud
```

### Adding Models

Generate a new database model:
```bash
rocket-cli generate model User name:String email:String
```

## Deployment

### Docker

Build and run with Docker:
```bash
docker build -t test-redis .
docker run -p 8000:8000 test-redis
```

### Configuration

Configuration is handled through environment variables with the prefix `STREAMER_`:
- `STREAMER_REDIS_URL` - Redis connection string
- `STREAMER_REDIS_POOL` - Redis connection pool size
- `STREAMER_SECRET_KEY` - Secret key for encryption
