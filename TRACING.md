# Tracing notes (future work)

This is a very brief checklist of what to keep in mind when adding tracing to
this project.

## Where tracing belongs

- Initialize tracing in `server/src/main.rs` (bootstrap layer).
- Keep the domain (`state.rs`, `systems/*`) free of tracing subscriber setup and
  transport-specific baggage.

## What to instrument

- Server startup: config values (non-secret), bind address, tick rate.
- World task (`game.rs`): tick duration, input queue depth (if available),
  snapshot/event fan-out rate.
- Networking (`net.rs`): connection lifecycle (connect/disconnect), message
  counts, decode/encode failures, backpressure/dropped broadcasts.
- Lobby orchestration (`lobby.rs`): join/create, lobby lifetime, per-lobby task
  spawn/stop.

## Conventions to use

- Prefer structured fields over string logs:
  - `player_id`, `conn_id`, `lobby_id`, `tick`, `dt_ms`, `bytes_in`, `bytes_out`.
- Use spans for long-lived contexts:
  - One span per connection.
  - One span per lobby.
  - One span for the world task.

## Things to avoid

- Logging inside hot inner loops without sampling or level-gating.
- Logging secrets (tokens, auth headers) or large payloads.
- Letting tracing types leak into core domain APIs.

## Operational toggles (so tracing is usable)

- Use `RUST_LOG` / `EnvFilter` to control verbosity.
- Consider JSON logging for easy ingestion (later: OpenTelemetry export).
