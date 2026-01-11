# Tracing Deep Dive (Jet Raiders)

This document explains *how tracing is implemented* in Jet Raiders in more detail than
`TRACING.md`.

It covers:

- Which libraries are used and why.
- How tracing is initialized and why it must happen early.
- Where the tracing subscriber lives (and what that means in Rust).
- How spans are used for correlation (connection → player identity).
- How different error paths flow through the code and get traced.
- What the program outputs and how to review it.

## Quick pointers to code

- Tracing initialization (subscriber + panic hook): `server/src/main.rs` (`init_tracing`).
- Connection span and lifecycle logs: `server/src/net.rs` (`handle_socket`).

## Libraries used (and why)

Jet Raiders uses Rust's `tracing` ecosystem rather than ad-hoc `println!` logging.

### `tracing`

Crate: `tracing`

Used for:

- Emitting **events** (`info!`, `warn!`, `error!`, etc.) with **structured fields**.
- Creating **spans** (`info_span!`) that attach context to all nested events.

Why this crate:

- Structured logs (fields like `player_id=123`) are machine-friendly and easier to
  filter/aggregate than parsing strings.
- Spans provide correlation context without manually repeating identifiers in every
  log call.

### `tracing-subscriber`

Crate: `tracing-subscriber`

Used for:

- Installing a global **subscriber** that decides *where* tracing output goes and
  *how* it is formatted.
- Configuring filtering at runtime via `EnvFilter` (driven by `RUST_LOG`).
- Choosing formatting:
  - Human-readable compact logs for local dev.
  - JSON logs for ingestion/search tooling.

Why this crate:

- It's the standard "batteries included" subscriber for `tracing`.
- `EnvFilter` is widely used and matches production patterns.

## What "initializing tracing" means in this codebase

In Rust, `tracing` itself is mostly a set of macros and APIs that emit events/spans.
Nothing is printed unless a **subscriber** is installed.

Jet Raiders installs that subscriber exactly once, at process startup:

- File: `server/src/main.rs`
- Function: `init_tracing()`
- Called at the beginning of `main()` before spawning tasks.

### Why initialization happens at startup

- The subscriber is a **global** resource (global dispatcher) for the process.
- `tracing_subscriber::fmt().init()` can only be called once; later calls will
  error/panic depending on configuration.
- Tasks spawned after initialization inherit the dispatcher automatically.

### Current initialization logic

`init_tracing()` does three important things:

1. **Installs a runtime filter** using `RUST_LOG` (via `EnvFilter`).
2. **Selects output format** (compact vs JSON) via `LOG_FORMAT=json`.
3. **Installs a panic hook** that logs a final structured `error!` event with a
   captured backtrace.

#### Filtering via `RUST_LOG`

Code:

- `tracing_subscriber::EnvFilter::try_from_default_env()`
- Fallback: `EnvFilter::new("info")`

Implications:

- If `RUST_LOG` is unset, you get `info` and above.
- If `RUST_LOG` is set, you can enable more verbosity per module (see examples
  below).

#### Output format via `LOG_FORMAT`

- Default: compact formatter (`.compact()`).
- If `LOG_FORMAT=json`: JSON formatter (`.json()`), and it also enables
  `.with_current_span(true)`.

`with_current_span(true)` is important because it includes the active span context
(e.g. `conn{conn_id=..., player_id=...}`) directly in each event record.

#### Panic hook

Rust panics are not regular `Result` errors, so they won’t automatically show up in
the normal error paths.

Jet Raiders installs:

```rust
std::panic::set_hook(Box::new(|info| {
    let backtrace = std::backtrace::Backtrace::capture();
    tracing::error!(%info, ?backtrace, "panic");
}));
```

This ensures a panic produces a final log event that includes:

- `info`: the panic message + location.
- `backtrace`: captured backtrace (when available).

## Where the tracing "object" lives

There is no `Tracing` struct passed around.

Instead:

- The process installs a **global subscriber** (via `tracing_subscriber`).
- `tracing::*` macros emit events/spans into the currently active dispatcher.
- The dispatcher is effectively "ambient context" (thread/task local, backed by a
  global default).

In practice, in this repo:

- The *configuration and ownership* of tracing lives in `server/src/main.rs`.
- The *usage* of tracing (events/spans) is allowed in outer layers like network and
  orchestration code (`server/src/net.rs`, `server/src/game.rs`).

This aligns with the Clean Architecture guidance in `TRACING.md`: domain code should
not depend on logging/tracing frameworks.

## Events vs spans (in depth)

### Events

An **event** is a point-in-time record.

Examples from `server/src/net.rs`:

- Connection lifecycle:

```rust
info!("client connected");
info!(player_id, "client disconnected");
```

- Invalid player input:

```rust
warn!(player_id, bytes = text.len(), error = %e, "failed to parse player input");
```

Key traits of good events in this codebase:

- Include stable identifiers as fields (`player_id`, `conn_id`, `missed`, `bytes`).
- Avoid logging unbounded payloads (e.g. do not log full JSON bodies).
- Use log levels consistently (`warn` for suspicious-but-recoverable conditions).

### Spans

A **span** represents a period of time and a logical scope.

In Jet Raiders, the most important span today is the per-connection span:

- Created when a WebSocket connection is upgraded.
- Entered for the rest of the connection handler.

Code (`server/src/net.rs`):

```rust
let conn_id = rand_id();
let span = info_span!("conn", conn_id, player_id = tracing::field::Empty);
let _enter = span.enter();
```

This span starts with:

- `conn_id`: always known.
- `player_id`: initially empty.

After the identity handshake completes, the span is updated:

```rust
span.record("player_id", ctx.player_id);
```

#### Why this span design matters

This pattern solves a real correlation problem:

- Early connection logs happen **before** the server has assigned a `player_id`.
- Later logs need `player_id` to correlate game behavior.

By using:

- `conn_id` as early correlation
- then recording `player_id` into the same span

…you can query logs by either identifier and still see the full connection story.

#### How span context is attached to logs

When a span is entered (`let _enter = span.enter();`), all events emitted inside
that scope become children of the span.

Depending on formatter:

- Compact logs may display span context implicitly (format-dependent).
- JSON logs include the current span explicitly due to
  `.with_current_span(true)`.

## How errors flow through the code (and get traced)

The server uses a mix of `Result` returns and local logging. The core idea is:

- Log *at the boundary where you have the most context*.
- Return errors upward when callers can add context or decide policy.

### 1) Startup errors (binding / serving)

File: `server/src/main.rs`

- Bind failure logs and then aborts startup:

```rust
tracing::error!(%addr, error = %e, "failed to bind");
return;
```

- Serve failure logs but does not panic:

```rust
tracing::error!(error = %e, "server error");
```

Why this is good:

- The log includes `addr` for bind failures.
- It avoids panics for operational errors.

### 2) Connection bootstrap errors

File: `server/src/net.rs`

Path:

- `handle_socket` calls `bootstrap_connection(...).await`.
- If it errors, the error is logged once with the connection span active:

```rust
error!(error = ?e, "failed to bootstrap connection");
```

Notes:

- The `?e` formatting emits the full debug representation of the enum error.
- Because we are inside the `conn` span, the output is correlated with `conn_id`.

Bootstrap can fail on:

- WebSocket send errors (`NetError::Ws`).
- Serialization errors (`NetError::Serialization`).
- Input channel closed (`NetError::InputClosed`).

### 3) Main client loop exits with error

File: `server/src/net.rs`

Path:

- `run_client_loop` returns `Result<(), NetError>`.
- Caller logs the final error at `warn` level:

```rust
warn!(error = ?e, "client loop exited with error");
```

This is intentionally not `error` today because many disconnect causes are expected
in a networked game (client closes tab, transient network loss, etc.).

### 4) Parse errors: malformed client input

File: `server/src/net.rs` (`handle_incoming_ws`)

When JSON input fails to parse:

```rust
warn!(player_id, bytes = text.len(), error = %e, "failed to parse player input");
```

Why this log is structured well:

- `bytes` provides a sanity check without logging the payload.
- `%e` prints the human-readable `serde_json` error message.

### 5) Backpressure: input channel full

When the input channel is full:

```rust
warn!(player_id, "input channel full; dropping input");
```

This is a *signal* that the world task may be overloaded or the client is spamming.
If this becomes noisy under load, it should be sampled/rate-limited (not yet
implemented).

### 6) Broadcast lag: world updates lagged

If a client lags behind on broadcast messages:

```rust
warn!(missed = n, "world updates lagged");
```

This log is important for diagnosing slow clients and/or server overload.

### 7) Watch channel closed: server state sender dropped

When `server_state_rx.changed()` errors:

- The code logs:

```rust
error!(error = ?e, "server state closed");
```

- And then marks the connection fatal (`NetError::ServerStateClosed`).

### 8) Panic path

If any code panics, the panic hook emits:

- `error!(%info, ?backtrace, "panic")`

This is your last-resort breadcrumb when a panic escapes normal error handling.

## What output you get (and how to review it)

Tracing output goes to standard output/error (depending on runtime and formatter).

### Compact output (default)

Intended for humans while developing locally.

Example commands:

```bash
RUST_LOG=info cargo run
RUST_LOG=server=debug cargo run
```

Tips for reading:

- Search for `conn_id` or `player_id` to follow a specific connection.
- Use `RUST_LOG=server=debug` temporarily when debugging connection handling.

### JSON output (`LOG_FORMAT=json`)

Intended for tooling (search, aggregation) and for reliably seeing span context.

Example:

```bash
LOG_FORMAT=json RUST_LOG=info cargo run
```

How to review JSON logs locally:

```bash
LOG_FORMAT=json RUST_LOG=info cargo run | jq -c '.'
```

Common workflows:

- Filter by a field:

```bash
LOG_FORMAT=json RUST_LOG=info cargo run | jq -c 'select(.fields.player_id == 123)'
```

- Filter by message substring:

```bash
LOG_FORMAT=json RUST_LOG=info cargo run | jq -c 'select(.message | contains("failed"))'
```

Note: The exact JSON shape is determined by `tracing_subscriber::fmt().json()`.
The important part is that fields like `player_id`, `conn_id`, and `missed` are
emitted as proper JSON fields rather than baked into a formatted string.

## Recommended tracing patterns to follow in this repo

- Log at boundaries:
  - network edge (`server/src/net.rs`)
  - process orchestration / startup (`server/src/main.rs`)
  - game loop orchestration (`server/src/game.rs`)
- Prefer fields over string interpolation.
- Use spans for stable correlation context (connection/session/lobby).
- Keep tick-level logs off by default (use `debug`/`trace` and guard against noise).

## Future extensions (non-breaking)

These are consistent with the current design and with the TODOs already in code:

- Add per-connection counters (messages/bytes in/out) and sample them.
- Add spans for higher-level gameplay contexts (e.g. `lobby`, `match`).
- Add tick timing instrumentation with sampling.
- Consider OpenTelemetry export when distributed tracing becomes necessary.
