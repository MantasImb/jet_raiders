# Binary Protocol Migration (Input + WorldUpdate)

This document explains how to migrate the per-tick input and world snapshot
messages from JSON text to binary WebSocket frames while keeping the handshake
and game state messages in JSON.

## 1. Scope

We will change only these message paths:

- Client -> Server: `PlayerInput` (per-tick input).
- Server -> Client: `WorldUpdate` (per-tick world snapshot).

We will keep these messages as JSON text:

- `Identity` (handshake).
- `GameState` (match state changes and resync snapshots).

This keeps the initial connection and state recovery readable while reducing
CPU and bandwidth for the high-frequency paths.

## 2. Motivation

JSON is easy to debug but expensive for per-tick traffic. Binary frames reduce
payload size and avoid JSON parsing on the hot path. This does not change tick
rate or network RTT, but it reduces per-message overhead and helps under load.

## 3. Wire Format (Binary Frames)

We will send binary frames for input and world updates. The WebSocket frame
already carries the length, so we only need a minimal envelope for type and
versioning.

Proposed envelope (all little-endian):

- `u8` version (start with `1`).
- `u8` kind:
  - `1` = Client Input
  - `2` = World Update
- `payload` (binary-serialized struct)

Payload serialization should live in `protocol.rs` and use a binary codec such
as `bincode` or `rkyv`. Pick one and keep it consistent across client and
server. (If we choose `bincode`, enable deterministic settings and avoid
variable-length floats.)

## 4. Server Changes

### 4.1 `protocol.rs`

Add binary DTOs and codec helpers. Keep JSON DTOs unchanged.

- `BinaryClientMessage::Input(PlayerInput)`
- `BinaryServerMessage::WorldUpdate(WorldUpdate)`
- Encode/decode helpers that add the `version` and `kind` envelope.

Keep the clean architecture boundaries:

- Protocol types stay in `protocol.rs`.
- `state.rs` and `systems/*` remain free of serialization details.

### 4.2 `net.rs`

Update the WebSocket loop:

- Accept `Message::Binary` for input.
- Decode the envelope and payload into `PlayerInput`.
- Reject binary messages with unknown `version` or `kind`.
- Reject JSON input once the client is migrated (or allow both during
  transition with a feature flag).

For outbound traffic:

- Send `WorldUpdate` as `Message::Binary` using the envelope format.
- Keep `Identity` and `GameState` as `Message::Text` JSON.

The resync path (`GameState` on lag) remains JSON because it is not per-tick
and is useful for readability and diagnostics.

## 4. Schema Ownership and Cross-Language Alignment

Binary serialization must be defined by an explicit layout, not by language
struct order or a serializer's default behavior. The layout should live in this
document (or a dedicated protocol spec file) and both the Rust server and the
Godot client should implement it byte-for-byte.

Guidelines:

- Treat this document as the canonical schema.
- Define field order and sizes explicitly.
- Keep endian fixed (little-endian).
- Use `u8` for booleans (`0` or `1`).
- Use counted arrays with a fixed-width count (`u16` unless you expect more
  than 65k items).
- Add golden test vectors (hex payloads) so both sides can assert that they
  decode the same bytes.

### 4.1 Binary Layout (Proposed)

Envelope (all little-endian):

- `u8` version
- `u8` kind (`1` = Input, `2` = WorldUpdate)

`PlayerInput` payload (kind = `1`):

- `f32` thrust
- `f32` turn
- `u8` shoot (`0` or `1`)

`WorldUpdate` payload (kind = `2`):

- `u64` tick
- `u16` entity_count
- `u16` projectile_count
- `EntityState[entity_count]`
- `ProjectileState[projectile_count]`

`EntityState`:

- `u64` id
- `f32` x
- `f32` y
- `f32` rot
- `i32` hp

`ProjectileState`:

- `u64` id
- `u64` owner_id
- `f32` x
- `f32` y
- `f32` rot

If we ever exceed `u16` counts, switch both sides to `u32` and bump the
envelope version.

## 5. Client Changes

Update the Godot client networking layer:

- Use binary encoding for `PlayerInput` and send as binary frames.
- Decode binary `WorldUpdate` frames.
- Keep JSON handling for `Identity` and `GameState`.

If the client needs to detect message type, branch on frame type:

- Text frames -> JSON (`Identity`, `GameState`).
- Binary frames -> binary envelope (`Input`, `WorldUpdate`).

### 5.1 Godot Serialization Notes

Use `StreamPeerBuffer` (or `PackedByteArray` with manual packing) to guarantee
little-endian encoding and correct field order. Avoid implicit serialization in
GDScript because it may not preserve sizes or layout between engine versions.

Minimal approach:

- Set `buffer.big_endian = false`.
- Write fields in the exact order from the schema.
- Read counts first, then loop and read fixed-size structs.
- Convert booleans to `u8` (`0` or `1`) on write and back on read.

For safety, add a small test script that encodes a known input and compares the
byte output against a golden hex vector from the Rust tests.

## 6. Migration Plan

Recommended phased rollout:

1. **Dual-stack** (optional): Server accepts both JSON input and binary input.
2. **Client update**: Ship binary input and binary world update decoding.
3. **Lockdown**: Remove JSON input handling when all clients are updated.

If you want to skip dual-stack, update server and client together and keep the
binary-only input path from day one.

## 7. Testing and Validation

- Add round-trip encode/decode tests for `PlayerInput` and `WorldUpdate` in
  `protocol.rs`.
- Compare payload sizes (JSON vs binary) for a typical snapshot.
- Verify that lag resync still works and emits JSON `GameState`.

## 8. Risks and Notes

- Binary formats are less readable; keep JSON for low-frequency and critical
  messages (`Identity`, `GameState`).
- Version the binary envelope to allow forward compatibility.
- Make sure the client and server use the same endian and codec settings.
