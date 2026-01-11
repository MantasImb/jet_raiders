# Clean Architecture Separation Guide (How to do it)

This document is an action-oriented guide for separating logic out of
`server/src/main.rs` into the modules described in `ARCHITECTURE.md`, while
respecting the boundaries defined in `CLEAN_ARCHITECTURE_GUIDELINES.md`.

This is intentionally simpler than the longer playbooks; use it as your primary
"how do I approach this refactor?" reference.

For additional examples and longer reference snippets, see
`CLEAN_ARCHITECTURE_SEPARATION_APPENDIX.md`.

## The target end-state (what "good" looks like)

Your server should read like three collaborating parts:

- Domain (the game brain): `state.rs`, `systems/*`, core world rules.
- Application (the world runner): `game.rs` (and later `lobby.rs`).
- Adapters/bootstrap (the glue): `net.rs`, `protocol.rs`, `main.rs`.

A quick check:

- If it **changes the world**: it does not belong in `main.rs` or `net.rs`.
- If it **talks WebSockets / JSON / Axum**: it does not belong in `game.rs`,
  `state.rs`, or `systems/*`.
- If it **wires channels / spawns tasks / starts the server**: it belongs in
  `main.rs` (or `lobby.rs` when lobbies exist).

## File-by-file responsibilities (use this when moving code)

- `main.rs`
  - Owns: config/tracing init, creating channels, spawning tasks, building router,
    starting server.
  - Does not own: WebSocket loops, serialization, tick loop, world mutation.

- `protocol.rs`
  - Owns: wire message types (`ClientMessage`, `ServerMessage`, `GameEvent`) and
    serde.
  - Does not own: `GameState` internals or game rules.

- `net.rs`
  - Owns: Axum router, WS upgrade, per-connection read/write loops, converting
    wire messages to typed domain inputs.
  - Does not own: ship movement/projectiles/collision, tick loop, authoritative state.

- `game.rs`
  - Owns: the tick loop; drains typed inputs; calls systems; emits
    snapshots/events.
  - Does not own: Axum/WebSocket types, JSON parsing.

- `state.rs`
  - Owns: the authoritative world model (`GameState`, entities, domain math).
  - Does not own: wire DTOs, serde, socket/task/channel wiring.

- `systems/*`
  - Owns: pure-ish update functions over `GameState`.
  - Does not own: networking, serde, spawning tasks.

- `lobby.rs` (when you have lobbies)
  - Owns: lobby registry, per-lobby channel wiring, spawning a game loop per
    lobby.
  - Does not own: Axum handlers or socket read/write.

## The wiring model (single-world)

You want this dataflow:

- WebSocket connections turn client messages into **typed inputs** and send them
  into a channel.
- The game loop drains inputs, updates `GameState`, and produces
  snapshots/events.
- Networking broadcasts snapshots/events back to clients.

A common shape:

- Input: many producers → one consumer (`mpsc::Sender` → `mpsc::Receiver`).
- Output: one producer → many consumers (typically `broadcast::Sender`).

Important: a single `mpsc::Receiver` cannot be cloned for multiple clients.

## Separation approach (a simple, safe order)

Do the refactor in small compiling moves. The high-level order below keeps
boundaries clear and minimizes churn.

### 1) Extract the protocol surface (`protocol.rs`)

Move wire-level message enums/structs out of `main.rs`:

- `ClientMessage`, `ServerMessage`, `GameEvent`
- Any serde derives and JSON-centric shapes

Rule: protocol types are a contract; keep them stable and keep them out of the
world model.

### 2) Extract the domain model (`state.rs`)

Move authoritative world types out of `main.rs`:

- `GameState`
- `Player`, `Projectile`, etc.

Rule: `state.rs` should not import Axum or serde.

### 3) Extract systems (`systems/*`)

Move world update logic into focused functions:

- `systems::ship_movement::update(state, dt)`
- `systems::projectiles::update(state, dt)`

Rule: systems should only depend on domain types.

### 4) Extract the tick loop (`game.rs`)

Move the long-running world loop out of `main.rs`:

- Own `GameState` in `game.rs`.
- Drain typed inputs from a channel.
- Call systems.
- Emit snapshots/events.

Rule: `game.rs` is allowed to know about time and channels; it should not know
about sockets.

### 5) Extract networking (`net.rs`)

Move WS code out of `main.rs`:

- Route + WS upgrade.
- Per-connection read/write loops.
- Translate `protocol::ClientMessage` → typed domain input.
- Write `protocol::ServerMessage` back to the socket.

Rule: `net.rs` should never call `systems::*`.

### 6) Reduce `main.rs` to wiring only

At the end, `main.rs` should do only:

- init tracing/config
- create channels
- spawn the game loop
- build router via `net::router(...)`
- start the server

If `main.rs` contains a loop that updates the world, it belongs in `game.rs`.

## Boundary conversions (where translation should happen)

Conversions should happen at boundaries:

- `net.rs`: deserialize/parse `protocol::ClientMessage` and convert to a typed
  domain input struct (`game::DomainInput` or similar).
- `net.rs` (or `protocol.rs` helpers): convert a domain snapshot/event into a
  `protocol::ServerMessage`.

Avoid:

- Using `protocol::*` types inside `state.rs`.
- Having systems accept protocol DTOs.

## Quick "am I done?" checklist

You’re separated correctly when:

- `main.rs` contains no world updates and no serde/JSON work.
- `net.rs` contains no calls into ship movement/projectile systems.
- `state.rs` and `systems/*` import no Axum/WebSocket/serde.
- `protocol.rs` imports no `GameState`.

## If you’re adding lobbies

Apply the same separation, but move orchestration to `lobby.rs`:

- `net.rs` selects a lobby (e.g., join message) and routes inputs to that lobby.
- `lobby.rs` owns per-lobby channels and spawns `game.rs` loops.
- `game.rs` stays focused on "one world loop" and does not manage lobby maps.
