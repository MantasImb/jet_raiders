# Process Compose

## Purpose

Use `process-compose` to run all Jet Raiders services in a single terminal. The
configuration uses `cargo watch` to re-run active Rust services when files
change.

## Usage

```bash
process-compose up
```

## Local Bootstrap

Before running `process-compose up`, create per-service `.env` files:

```bash
cp auth_server/.env.example auth_server/.env
cp game_server/.env.example game_server/.env
cp head_server/.env.example head_server/.env
cp matchmaking_server/.env.example matchmaking_server/.env
```

Set local-first values in those files:

- `auth_server/.env`
  - `AUTH_SERVER_BIND_HOST=127.0.0.1`
  - `DATABASE_URL=postgres://<user>:<pass>@127.0.0.1:5432/<db>`
  - `BACKEND_PORTS_CONFIG_PATH=../config/backend_ports.toml`
  - `AUTH_SERVER_PORT=` (optional override; exact empty string uses file port)
- `game_server/.env`
  - `GAME_SERVER_BIND_HOST=127.0.0.1`
  - `GAME_SERVER_PORT=3001`
  - `AUTH_SERVICE_URL=http://127.0.0.1:3002`
- `head_server/.env`
  - `HEAD_SERVER_BIND_HOST=127.0.0.1`
  - `BACKEND_PORTS_CONFIG_PATH=../config/backend_ports.toml`
  - `HEAD_SERVER_PORT=` (optional override; exact empty string uses file port)
  - `AUTH_SERVICE_URL=http://127.0.0.1:3002`
  - `MATCHMAKING_SERVICE_URL=http://127.0.0.1:3003`
  - `REGION_CONFIG_PATH=../config/regions.toml`
- `matchmaking_server/.env`
  - `MATCHMAKING_SERVER_BIND_HOST=127.0.0.1`
  - `BACKEND_PORTS_CONFIG_PATH=../config/backend_ports.toml`
  - `MATCHMAKING_SERVER_PORT=` (optional override; exact empty string uses
    file port)
  - `REGION_CONFIG_PATH=../config/regions.toml`

## Active services

- `auth-server`: runs in `auth_server` via `cargo watch -x check -x run`
- `game-server`: runs in `game_server` via `cargo watch -x check -x run`
- `head-server`: runs in `head_server` via `cargo watch -x check -x run`
- `matchmaking-server`: runs in `matchmaking_server` via
  `cargo watch -x check -x run`

## Inactive entries in config

- `website` is currently commented out and remains a placeholder.
- Uncomment and configure this entry when you want it in the local
  process-compose stack.

## Notes

- Run `process-compose` from the repository root so `working_dir` paths resolve
  correctly.
- The current setup depends on `cargo watch` being installed.
