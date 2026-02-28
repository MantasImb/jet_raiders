# Tracing (Jet Raiders)

This document defines how and where tracing is implemented in Jet Raiders so
bugs and crashes can be diagnosed quickly.

The design follows common production patterns in Rust:

- `tracing` events with structured fields (not string parsing)
- spans for long-lived context (connection lifecycle)
- `EnvFilter` (`RUST_LOG`) for runtime control
- optional JSON logs for ingestion by log tooling

## Goals

1. Fast root-cause: every error log should include identifiers needed to locate
   the responsible player/connection/tick.
2. Correlation: logs must be groupable by stable identifiers (for example
   `conn_id`, `player_id`, `lobby_id`).
3. Safety: do not log secrets or unbounded payloads.
4. Low noise: avoid spam in hot paths by using levels and throttled warnings.

## Where tracing lives (Clean Architecture alignment)

Tracing is an outer-layer concern.

- Allowed:
  - `game_server/src/frameworks/server.rs`
  - `game_server/src/interface_adapters/net/client.rs`
  - `game_server/src/interface_adapters/net/internal.rs`
  - `game_server/src/use_cases/game.rs`
- Avoid in domain:
  - `game_server/src/domain/state.rs`
  - `game_server/src/domain/systems/*`

Rule: domain code should not be forced to depend on a logging framework.

## Current implementation

### 1) Subscriber initialization (bootstrap)

File: `game_server/src/frameworks/server.rs`

Current behavior:

- Tracing is initialized once during startup (`init_runtime()`).
- `RUST_LOG` controls filtering through `EnvFilter`.
- `LOG_FORMAT=json` enables JSON output.
- Panic hook logs panic info and backtrace as a structured error event.

### 2) Connection lifecycle spans (network adapter)

File: `game_server/src/interface_adapters/net/client.rs`

Current behavior:

- A connection span is created immediately on upgrade.
- `conn_id` is available before identity is known.
- `player_id` is recorded into the span after successful join/auth handshake.
- Disconnect cleanup emits per-connection stats at `debug`.

Representative snippet:

```rust
let conn_id = rand_id();
let span = info_span!("conn", conn_id, player_id = tracing::field::Empty);
let _enter = span.enter();
span.record("player_id", ctx.player_id);
```

Tracked connection stats:

- `msgs_in`
- `msgs_out`
- `bytes_in`
- `bytes_out`
- `invalid_json`
- `lag_recovery_count`

### 3) World and lobby lifecycle events

Files:

- `game_server/src/use_cases/game.rs`
- `game_server/src/interface_adapters/net/client.rs`

Current behavior:

- `info` for lifecycle milestones (listen, connect/disconnect, join/leave).
- `warn` for suspicious recoverable behavior (lagged updates, invalid input,
  full input channels).
- warning logs in hot paths are rate-limited with a shared throttle window.

Representative snippet:

```rust
info!(player_id, "player joined");
info!(player_id, "player left");
```

## Rules and conventions (non-negotiable)

### Prefer structured fields

Bad:

```rust
println!("player {} disconnected", player_id);
```

Good:

```rust
tracing::info!(player_id, "client disconnected");
```

### Do not log unbounded payloads

- Do not log full client payloads in parse/auth error paths.
- Prefer bounded metadata like `bytes = text.len()` and an error summary.

### Levels

- `error`: unrecoverable errors, panics, startup/bind failures
- `warn`: recoverable but suspicious conditions (lag, dropped input, parse noise)
- `info`: lifecycle milestones
- `debug`: connection stats and verbose diagnostics
- `trace`: very high-volume instrumentation, only for focused local debugging

### No tracing setup outside bootstrap

- No `tracing_subscriber` usage outside
  `game_server/src/frameworks/server.rs`.

## Operational usage

Recommended local defaults:

```bash
RUST_LOG=info cargo run
```

More detail for networking:

```bash
RUST_LOG=game_server::interface_adapters::net::client=debug cargo run
```

JSON logs:

```bash
LOG_FORMAT=json RUST_LOG=info cargo run
```

Backtraces on panic:

```bash
RUST_BACKTRACE=1 RUST_LOG=info cargo run
```

## Next additions (planned)

- Tick timing telemetry (`tick_ms`) with sampling.
- Add `lobby_id` as a span field on connection spans.
- Optionally emit periodic aggregate counters per lobby.
- Evaluate OpenTelemetry export when cross-service correlation is needed.
