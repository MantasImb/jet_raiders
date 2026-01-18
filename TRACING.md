# Tracing (Jet Raiders)

This document defines **how** and **where** tracing is implemented in Jet Raiders
so that bugs and crashes can be pinpointed quickly.

For an in-depth implementation guide, see `TRACING_DEEP_DIVE.md`.

The design is intentionally based on widely adopted production patterns in Rust:

- `tracing` events with **structured fields** (not string parsing)
- **spans** for long-lived context (connection, world loop)
- an `EnvFilter` (`RUST_LOG`) for runtime control
- optional **JSON logs** for easy ingestion in log tooling

## Goals

1. **Fast root-cause**: every error log should include the minimum identifiers
   needed to find the responsible player/connection/tick.
2. **Correlation**: logs must be groupable by a stable identifier (e.g. `conn_id`,
   `player_id`).
3. **Safety**: do not log secrets or unbounded payloads.
4. **Low noise**: avoid spam in hot loops; use levels and sampling.

## Where tracing lives (Clean Architecture alignment)

Tracing is an *outer-layer concern*.

- ✅ Allowed: `game_server/src/main.rs`, `game_server/src/net.rs`, `game_server/src/game.rs`
- 🚫 Avoid in domain: `game_server/src/state.rs`, `game_server/src/systems/*`

Rule: domain code should not be forced to depend on a logging framework.

## Current implementation

### 1) Subscriber initialization (bootstrap)

File: `game_server/src/main.rs`

Principles:

- Initialize tracing once at process startup.
- Use `RUST_LOG` for dynamic verbosity control.
- Optional JSON output via `LOG_FORMAT=json`.
- Install a panic hook to emit a final structured error event.

Key snippet:

```rust
fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let json = matches!(std::env::var("LOG_FORMAT").as_deref(), Ok("json"));
    if json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .json()
            .with_current_span(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .compact()
            .init();
    }

    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::capture();
        tracing::error!(%info, ?backtrace, "panic");
    }));
}
```

### 2) Connection lifecycle spans (network adapter)

File: `game_server/src/net.rs`

Principles:

- Create a **connection span** immediately on upgrade.
- Use `conn_id` to correlate pre-auth / pre-identity logs.
- Record `player_id` into the span after the Identity handshake.

Key snippet:

```rust
let conn_id = rand_id();
let span = info_span!("conn", conn_id, player_id = tracing::field::Empty);
let _enter = span.enter();

// ... bootstrap assigns player_id ...
span.record("player_id", ctx.player_id);
```

Errors should log:

- `conn_id`
- `player_id` (if known)
- `error` details
- relevant counts (e.g. `bytes`, `missed`)

### 3) World task events (game loop)

File: `game_server/src/game.rs`

Principles:

- Log coarse lifecycle events at `info` (join/leave, state transitions).
- Log unusual situations at `warn` (lag, dropped inputs).
- Keep per-tick logging at `debug/trace` only.

Examples:

- Player join/leave:

```rust
info!(player_id, "player joined");
info!(player_id, "player left");
```

- Projectile hit (currently no health system; this is for debugging/correlation):

```rust
info!(
    victim_id = e.id,
    shooter_id = p.owner_id,
    projectile_id = p.id,
    "player hit"
);
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

- Do not log full JSON payloads in error paths.
- Prefer `bytes = payload.len()` and the parse error.

### Levels

- `error`: unrecoverable errors, panics, task exits that impact gameplay
- `warn`: recoverable but suspicious (lagged broadcasts, dropped input)
- `info`: lifecycle milestones (listen, connect/disconnect, join/leave, hits)
- `debug`: useful for local debugging (non-text ws messages)
- `trace`: extremely verbose (tick-level instrumentation)

### No tracing setup outside bootstrap

- No `tracing_subscriber` usage outside `game_server/src/main.rs`.

## Operational usage

### Recommended local defaults

Compact logs:

```bash
RUST_LOG=info cargo run
```

More detail for debugging networking:

```bash
RUST_LOG=server=debug cargo run
```

JSON logs:

```bash
LOG_FORMAT=json RUST_LOG=info cargo run
```

Backtraces on panic:

```bash
RUST_BACKTRACE=1 RUST_LOG=info cargo run
```

## What to add next (planned)

- Per-connection counters (messages in/out, bytes in/out).
- Tick timing metrics (`tick_ms`) with sampling.
- A `lobby_id` span once lobbies are wired in.
- OpenTelemetry export once we need distributed tracing.
