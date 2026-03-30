# tinistreamer

A lightweight streaming broker that relays events from a backend to frontend clients in real time. Your backend creates a named stream and writes events into it; clients connect via SSE or WebSocket and a secure token, and receive those events live. Ideal for handling short-lived streams, such as LLM streaming responses and other real-time applications. Used in [RsChat](https://github.com/fa-sharp/rs-chat), a self-hostable LLM chat app.

The service is built on [Redis Streams](https://redis.io/docs/latest/develop/data-types/streams/). It is stateless — no persistence beyond Redis TTLs.

## Flow

1. Backend calls `POST /api/stream/` to create a stream and receive a short-lived client token.
2. Backend writes events via regular HTTP requests, newline-delimited JSON streaming, or WebSocket.
3. Frontend clients connect to the stream via SSE or WebSocket using the secure token, receiving all past and future events.

## Running

**Local:**
```bash
cd api
cp .env.example .env  # set STREAMER_API_KEY and STREAMER_SECRET_KEY
cargo run
```

**Docker Compose** (includes Redis):
```bash
docker compose up
```

**Docker Run:**
```bash
docker build -t tinistreamer .
docker run -p 8080:8080 \
  -e STREAMER_API_KEY=your-key \
  -e STREAMER_SECRET_KEY=64-char-hex \
  -e STREAMER_REDIS_URL=redis://host:6379 \
  tinistreamer
```

## Configuration

All settings can be provided via `Rocket.toml` or `STREAMER_*` environment variables.

| Env var | Default | Description |
|---|---|---|
| `STREAMER_API_KEY` | required | API key for backend authentication |
| `STREAMER_SECRET_KEY` | required | 64-char hex string for client token encryption (AES-256-GCM) |
| `STREAMER_REDIS_URL` | `redis://localhost:6379` | Redis connection string |
| `STREAMER_SERVER_ADDRESS` | `http://localhost:8000` | Public URL used to build SSE/WS client URLs |
| `STREAMER_TTL` | `600` | Stream and token TTL in seconds |
| `STREAMER_CLIENT_TIMEOUT` | `300` | Seconds a client connection can be idle before being dropped |
| `STREAMER_REDIS_POOL` | `4` | Static Redis connection pool size (for writes/management) |
| `STREAMER_MAX_CLIENTS` | `20` | Max concurrent streaming client connections |
| `STREAMER_ALLOWED_ORIGINS` | all | Comma-separated CORS allowed origins |
| `STREAMER_ADDRESS` | `127.0.0.1` | Bind address |
| `STREAMER_PORT` | `8000` | Bind port |

## API

All management and ingestion routes require the `X-API-KEY` header. Client consumer routes use a short-lived bearer token obtained when creating a stream.

### Health & Info

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/health` | Health check, returns `"OK"` |
| `GET` | `/api/info` | Server version and Redis pool stats |

### Stream Management

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/stream/` | List active streams (optional `?pattern=` to filter the streams by key) |
| `GET` | `/api/stream/info` | Get length and TTL for a stream (`?key=`) |
| `GET` | `/api/stream/events` | Fetch all stored events from a stream (`?key=`) |
| `POST` | `/api/stream/` | Create a stream; returns `{ sse_url, ws_url, token }` |
| `POST` | `/api/stream/token` | Generate a new client token for an existing stream |
| `POST` | `/api/stream/end` | End a stream (writes `end` sentinel, notifies clients) |
| `POST` | `/api/stream/cancel` | Cancel a stream (writes `cancel` sentinel, notifies clients) |

### Event Ingestion

| Method | Path | Description |
|---|---|---|
| `POST` | `/api/event/add` | Add a batch of events: `{ key, events: [{ event, data? }] }` |
| `POST` | `/api/event/add/json-stream` | Stream newline-delimited JSON events (`?key=`); events are forwarded as they arrive |
| `GET` | `/api/event/add/ws-stream` | Add events via WebSocket (`?key=`); each message is `{ event, data? }` |

### Client Consumers

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/client/sse` | Subscribe to a stream via SSE (`?key=`); supports `Last-Event-ID` for reconnection |
| `GET` | `/api/client/ws` | Subscribe to a stream via WebSocket (`?key=`); first message is all prior events |

## Notes

- Streams are capped at 500 events in Redis (`XADD MAXLEN ~ 500`).
- Each SSE/WebSocket consumer holds a dedicated Redis connection for the life of the connection. New connections are rejected with `429` when the exclusive pool (`STREAMER_MAX_CLIENTS`) is exhausted.
- Client tokens embed an expiry and the stream key, encrypted with AES-256-GCM. They are validated on every request.
- Generated client libraries for Rust and Python are available in `clients/`.
