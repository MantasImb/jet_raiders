# Clean Architecture Guidelines (Jet Raiders)

This document consolidates the clean architecture guidance for this project.
It is the single source of truth for where server logic should live.

See `ARCHITECTURE.md` for the current module layout and any project-specific
exceptions.

## Dependency rule (non-negotiable)

Dependencies must point inward:

- Outer layers may depend on inner layers.
- Inner layers must not depend on outer layers.

In practice, that means:

- `domain/state.rs` and `domain/systems/*` must not import Axum, WebSocket types, `serde`,
  `tokio::net`, or other transport/runtime/framework concerns.
- `interface_adapters/protocol.rs` must not import `GameState` or other authoritative world
  internals.
- `frameworks/server.rs` and `interface_adapters/net.rs` are allowed to import
  everything because they are the outermost wiring/adapter layer.

## Layer mapping for this repository

Use this mapping when deciding where code belongs:

- Entities (core domain): `domain/state.rs`, `domain/systems/*`, `domain/tuning/*`
- Use cases (application): `use_cases/game.rs`, `use_cases/lobby.rs`, `use_cases/types.rs`
- Interface adapters: `interface_adapters/net.rs`,
  `interface_adapters/protocol.rs`, `interface_adapters/state.rs`
- Frameworks/drivers: `frameworks/server.rs`, `frameworks/config.rs`, `main.rs`

## Module responsibilities (authoritative)

If you are unsure where something belongs, use the rules below.

### `frameworks/server.rs` (bootstrap)

Owns:

- Config and tracing init.
- Creating channels.
- Spawning tasks.
- Building the router and starting the server.

Must not own:

- WebSocket read/write loops.
- JSON encode/decode.
- Game rules or `GameState` mutation.

### `frameworks/config.rs` (configuration)

Owns:

- Reading env/config.
- Configuration types.

Must not own:

- Game rules.
- Networking.

### `interface_adapters/protocol.rs` (wire contract)

Owns:

- On-the-wire DTOs: `ClientMessage`, `ServerMessage`, input/output DTOs.
- `serde` derives and schema stability.

Must not own:

- `GameState` internals.
- Physics/game rules.
- Axum/WebSocket types.

### `interface_adapters/net.rs` (network adapter)

Owns:

- Axum router and WS upgrade.
- Per-connection read/write loops.
- Translating wire messages to typed domain inputs.

Must not own:

- Tick loop.
- Ship movement/projectile/collision logic.
- Authoritative `GameState`.

### `use_cases/lobby.rs` (application orchestration)

Owns:

- Lobby registry.
- Per-lobby channel wiring.
- Spawning game loops per lobby.

Must not own:

- Axum handlers.
- Socket read/write loops.
- Game rules inside systems.

### `use_cases/game.rs` (world task)

Owns:

- The tick loop.
- Draining typed inputs.
- Calling `systems/*`.
- Producing snapshots/events.

Must not own:

- Axum router.
- WebSocket types.
- JSON parsing.

### `domain/state.rs` (domain model)

Owns:

- Authoritative world model (`GameState`, entities, domain math/types).

Must not own:

- Wire DTOs.
- `serde`.
- Axum.
- Socket/channel plumbing.

### `domain/systems/*` (domain updates)

Owns:

- Update functions that mutate `GameState` (ship movement, projectiles, etc.).

Must not own:

- Networking.
- `serde`.
- Spawning tasks.
- Reading sockets.

## Boundaries: domain types vs protocol DTOs

This codebase separates two different kinds of data shapes:

- Domain types (authoritative): live in `domain/state.rs` and are mutated by
  `domain/systems/*` and orchestrated by `use_cases/game.rs`.
- Protocol DTOs (wire contract): live in `interface_adapters/protocol.rs` and exist only to
  serialize/deserialize and version the client/server message contract.

Rules:

- Do not store `protocol::*` types inside domain entities.
- Do not make systems accept `protocol::*` types.
- Convert at the boundary (usually `interface_adapters/net.rs`).

A good pattern is to have:

- `protocol::ClientMessage::Input(protocol::PlayerInput)` as the wire type.
- `game::DomainInput` (or `state::PlayerInputState`) as the domain type.

## Boundaries: networking vs game loop

The game loop is a long-running task that owns the authoritative world state.
Networking code never owns or mutates `GameState`.

- `interface_adapters/net.rs` receives bytes, parses them into `protocol::ClientMessage`, and
  converts to typed domain inputs.
- `use_cases/game.rs` receives typed domain inputs over channels, updates the world, and
  emits outputs.

### Outputs and snapshots

You have two acceptable shapes for outputs:

- Strict separation (preferred): `use_cases/game.rs` emits domain events/snapshots and
  `interface_adapters/net.rs` converts them to `protocol::ServerMessage`.
- Pragmatic early stage: `use_cases/game.rs` emits `protocol::ServerMessage` as long as
  `use_cases/game.rs` does not import Axum/WebSocket types.

Do not let protocol types leak into `domain/state.rs` or `domain/systems/*`
either way.

## Ownership and channel wiring

Channel and task ownership lives in the outer layers:

- `frameworks/server.rs` wires the app together (single world or lobby manager), then starts
  the server.
- `use_cases/lobby.rs` owns per-lobby channels and the lifetime of lobby game loop tasks.
- `interface_adapters/net.rs` owns per-connection tasks.

Notes:

- A single `mpsc::Receiver` cannot be cloned for fan-out to multiple
  connections.
  - If many clients need the same outgoing stream, use `tokio::sync::broadcast`
    or provide per-connection receivers.

## Single-world vs lobbies

Both are supported by the same boundaries; only the orchestration changes.

### Single-world

- One `use_cases/game.rs` loop owns one `GameState`.
- All connections send inputs into the same input channel.
- Outgoing snapshots/events are broadcast to all connections.

Where things live:

- `frameworks/server.rs` creates the channels and spawns the single game loop.
- `interface_adapters/net.rs` uses shared senders/receivers to connect each
  socket.

### Lobbies

- Each lobby has its own game loop task and its own channels.
- `use_cases/lobby.rs` returns a `LobbyHandle` (typically containing `tx_input` and an
  outgoing receiver/subscription).
- `interface_adapters/net.rs` selects a lobby based on a join message, then routes subsequent
  inputs to that lobby handle.

## Rules of thumb (fast routing for new code)

- If it changes the world: `use_cases/game.rs`, `domain/state.rs`,
  `domain/systems/*`.
- If it talks JSON/WebSockets or Axum types: `interface_adapters/net.rs` and
  `interface_adapters/protocol.rs`.
- If it creates tasks, channels, registries, or starts the server:
  `frameworks/server.rs` and `use_cases/lobby.rs`.

## Common mistakes to avoid

- Protocol DTOs used as domain state:
  - Symptom: `crate::protocol::*` imported in `domain/state.rs` or `domain/systems/*`.
  - Fix: introduce/keep a domain input/state type and convert in
    `interface_adapters/net.rs`.

- Game loop depends on sockets/framework:
  - Symptom: `use_cases/game.rs` imports Axum WebSocket types.
  - Fix: move socket logic to `interface_adapters/net.rs`; keep only
    channels/time/domain in `use_cases/game.rs`.

- Game loop logic in `frameworks/server.rs`:
  - Symptom: `frameworks/server.rs` contains a tick/update loop or mutates
    `GameState`.
  - Fix: move the loop into `use_cases/game.rs`; leave
    `frameworks/server.rs` as composition only.

- Fan-out using a single `mpsc::Receiver`:
  - Symptom: trying to clone the receiver or share it between connections.
  - Fix: `broadcast::Sender` for outgoing messages or per-connection channels.
