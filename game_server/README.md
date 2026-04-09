# Game Server

## Purpose

The game server owns authoritative real-time world simulation and player
session participation over WebSocket.

## HTTP API

- `GET /health`
  - Liveness endpoint for startup and container smoke checks.
- `POST /lobbies`
  - Creates a lobby for head-service handoff.
- `GET /ws?lobby_id=<id>`
  - Upgrades to the gameplay WebSocket for the selected lobby.

## Runtime and Configuration

- Required bind host env var: `GAME_SERVER_BIND_HOST`
- Required auth base URL env var: `AUTH_SERVICE_URL`
- Bind address: `<GAME_SERVER_BIND_HOST>:<GAME_SERVER_PORT>`
- Optional port env var: `GAME_SERVER_PORT` (default `3001`)
- Optional auth timeout env var: `AUTH_VERIFY_TIMEOUT_MS` (default `1500`)
- Tracing controls: `RUST_LOG`, optional `LOG_FORMAT=json`

Fatal startup exit codes:

- `1`: required startup config is missing.
- `2`: startup config is invalid.
- `3`: startup dependency initialization failed.
- `4`: service failed to bind its listener socket.
- `5`: server runtime failed while serving requests.

## Docker

Build the game image from the repository root:

```bash
docker build -f game_server/Dockerfile -t jet-raiders/game-server:phase2 .
```

Run the game container:

```bash
docker run --rm \
  --name game-server \
  -p 3001:3001 \
  -e GAME_SERVER_BIND_HOST=0.0.0.0 \
  -e GAME_SERVER_PORT=3001 \
  -e AUTH_SERVICE_URL="http://auth-server:3002" \
  jet-raiders/game-server:phase2
```

Smoke-check liveness from another terminal:

```bash
curl -sS http://127.0.0.1:3001/health
```

Expected response:

```json
{"status":"ok"}
```

Negative-path check for required bind-host config:

```bash
docker run --rm \
  --name game-server-missing-bind \
  -e AUTH_SERVICE_URL="http://auth-server:3002" \
  jet-raiders/game-server:phase2
echo $?
```

Expected outcome:

- Logs include `GAME_SERVER_BIND_HOST`.
- Process exits with status code `1`.

Negative-path check for required auth endpoint config:

```bash
docker run --rm \
  --name game-server-missing-auth-url \
  -e GAME_SERVER_BIND_HOST=0.0.0.0 \
  jet-raiders/game-server:phase2
echo $?
```

Expected outcome:

- Logs include `AUTH_SERVICE_URL`.
- Process exits with status code `1`.

## Testing

Run tests in the service directory:

```bash
cargo test
```
