# World Update Scaling Plan

This document outlines an incremental path from broadcasting identical world
updates to per-client visibility, while respecting clean architecture rules.

## Goals

- Reduce redundant serialization work today.
- Avoid sending hidden or irrelevant data as the world grows.
- Keep domain logic separate from transport and serialization.

## Phase 1: Serialize once, broadcast bytes

### Why

The current bottleneck is repeating the same serialization work for every
connection. We can remove that waste without changing what data is sent.

### What changes

- Keep `game.rs` emitting `WorldUpdate` as it does now.
- Add a single adapter task that:
  - Receives `WorldUpdate` once per tick.
  - Builds `ServerMessage::WorldUpdate`.
  - Serializes once.
  - Broadcasts shared bytes to all connections.
- Connection loops send the shared bytes directly.

### Clean architecture alignment

- Serialization stays in adapter/wiring code (`net.rs` or `main.rs`).
- `game.rs` and domain types do not import `serde` or socket types.

## Phase 2: Interest management and culling

### Why

When the world becomes large, a full snapshot is too big and may reveal
information a player should not see.

### What changes

- Introduce a world partitioning model in the domain layer
  (for example, grid cells or regions).
- Track player positions and visibility in the game loop.
- Emit a smaller, per-player "visible slice" descriptor from the game loop.
  This is still domain data, not network bytes.
- Adapter layer converts the slice into a `ServerMessage` and serializes per
  client (now payloads differ).

### Clean architecture alignment

- Visibility rules live in the game loop and domain types.
- Serialization remains in the adapter layer.

## Phase 3: Deltas, prioritization, and compression

### Why

Even per-client slices can be large. Deltas reduce bandwidth and improve
latency under load.

### What changes

- Track last-sent state per client (adapter layer or a dedicated cache task).
- Send deltas and prioritize critical events.
- Optional: add compression or binary protocol once the message shapes are
  stable.

### Clean architecture alignment

- Domain remains authoritative; adapter layer handles packaging and transport.

## Security considerations

- Do not send hidden entities or off-screen events to clients.
- Avoid sending server-only data in any protocol DTOs.
- Treat the client as untrusted; validate inputs in the game loop.

## Success criteria

- Phase 1: CPU and allocator pressure drop under many clients.
- Phase 2: Bandwidth scales with visible entities, not world size.
- Phase 3: Large battles remain stable under load and do not leak data.
