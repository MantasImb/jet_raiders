# Game Server Testing Notes

This document captures the current testing direction for `game_server`.
It is not a strict spec and not a loose summary. Treat it as practical guidance
for writing and extending endpoint tests in this service.

## What we test

We focus on black-box behavior:

- HTTP and WebSocket endpoints only.
- Status codes, JSON payloads, and protocol shape.
- Observable behavior from a client or internal service perspective.

We do not validate internal state in these tests.
Domain rules (physics, collisions, damage) should be covered in domain/system
tests under `game_server/src/domain/*`.

## Current endpoint surface

### HTTP

- `POST /lobbies`
  - Request: `{ "lobby_id": "...", "allowed_player_ids": [...] }`
  - Success: `201` with `{ "lobby_id": "..." }`
  - Errors:
    - `400` with `{ "error": "lobby_id is required" }`
    - `409` with `{ "error": "lobby already exists" }`

### WebSocket

- `GET /ws?lobby_id=...&player_id=...`
  - Missing lobby should return `404` with `{ "error": "lobby not found" }`

## Current test layout

```text
game_server/
  tests/
    lobby_create.rs
    ws_join.rs
    support/
      mod.rs
```

- `tests/support/mod.rs`: one-time server bootstrap and readiness checks.
- `tests/lobby_create.rs`: contract tests for lobby creation.
- `tests/ws_join.rs`: reserved for WebSocket join contract tests.

## Server startup model used by tests

The production and test startup paths are intentionally split:

- `run_with_config()` binds configured address and initializes runtime concerns
  (`dotenv`, tracing).
- `run(listener)` serves the app with a caller-provided `TcpListener`.

Tests should call `run(listener)` with an ephemeral listener bound to
`127.0.0.1:0`.
This avoids collisions if a local service is already using `3001`.

## One server per test binary

`tests/support/mod.rs` uses `OnceLock` to start exactly one server thread for
the whole test binary:

- Spawn a dedicated OS thread.
- Create a Tokio runtime in that thread.
- Bind `TcpListener` to `127.0.0.1:0`.
- Publish the resulting base URL in a global.
- Run `game_server::run(listener)`.
- Poll readiness against that exact published address.

This avoids two common failures:

- Starting a server in one test runtime and losing it in another runtime.
- Accidentally targeting a different process bound to a fixed port.

## How `tests/support/mod.rs` works

This helper is the core of endpoint-test reliability in this service.
It solves lifecycle issues that happen when each `#[tokio::test]` has its own
runtime.

### Globals and ownership

- `SERVER_READY: OnceLock<()>`
  - Guards one-time startup.
  - The server bootstrap closure runs only once per test binary.
- `SERVER_URL: OnceLock<String>`
  - Stores the final `http://127.0.0.1:<ephemeral-port>` base URL.
  - All tests read from this after startup completes.

### Startup sequence

1. A test calls `ensure_server()`.
2. `SERVER_READY.get_or_init(...)` enters startup only on the first call.
3. Startup creates a temporary `published_url: Arc<OnceLock<String>>`.
4. A dedicated OS thread is spawned.
5. The thread builds a Tokio runtime and calls `runtime.block_on(...)`.
6. Inside async startup, the server binds `TcpListener` to `127.0.0.1:0`.
7. The OS picks a free port; the code reads it from `listener.local_addr()`.
8. The thread publishes `http://<addr>` into `published_url`.
9. The thread runs `game_server::run(listener)` and serves forever.
10. The parent startup path waits until URL publication and TCP readiness.
11. The published URL is copied into global `SERVER_URL`.
12. `ensure_server()` returns `&'static str` from `SERVER_URL`.

### Why this design is used

- It avoids fixed-port collisions with local dev services.
- It keeps one shared server process per test binary.
- It prevents test-runtime teardown from killing the server unexpectedly.
- It makes all tests target the exact server they started.

### Readiness strategy

- Readiness is not based on a configured port.
- Readiness probes the exact published host:port (`TcpStream::connect`).
- This prevents false positives from unrelated services on common ports.

## Writing new endpoint tests

Use this pattern in each integration test:

1. `let base_url = support::ensure_server();`
2. Build a `reqwest::Client`.
3. Send request to `format!("{base_url}/...")`.
4. Assert status code first, then response JSON contract.

Practical rules:

- Generate unique lobby IDs per test (`uuid` is already in dev-dependencies).
- Keep assertions contract-focused (field names and values).
- Keep tests deterministic and independent of execution order.

## Recommended cases for `POST /lobbies`

- Create lobby with valid payload returns `201` and echoes `lobby_id`.
- Create same lobby twice returns `409` on second attempt.
- Create with empty/whitespace `lobby_id` returns `400`.
- Optional: create with `allowed_player_ids` and verify join behavior in WS
  tests.

## Running tests

From `game_server/`:

```bash
cargo test --tests -- --test-threads=1
```

Single-threaded execution is recommended for now to keep lifecycle predictable
while using one shared server per test binary.

## Limitations and next improvements

- Current teardown is process-lifetime based (no explicit graceful shutdown).
- If graceful shutdown is needed, add a shutdown signal to `run(listener)` and
  trigger it from test support at the end of the test process.
- Expand `ws_join.rs` with handshake and error contract tests next.
