# Client Networking Handling

This document describes how the Godot client handles the connection with the
Rust server, including input transmission, world state synchronization, and
match state management.

## 1. Connection Management

The client connects to the server using WebSockets (`WebSocketPeer` in Godot).

- **Server URL**: `ws://127.0.0.1:3000/ws`
- **Protocol**: JSON-based messages.

### Connection Lifecycle

1. **Connecting**: The `NetworkManager` initializes a `WebSocketPeer` and
   attempts to connect.
2. **Handshake**: Once connected, the server assigns a `player_id` (handled server-side).
3. **First state**: The server sends the initial state when the player joins, so that the client can display its own entity.
4. **Polling**: The client must call `socket.poll()` every frame (in `_process`)
   to process incoming packets and maintain the connection.

## 2. Sending Inputs

Inputs are sent from the client to the server to drive the simulation.

### Input Packet Structure

The server expects a JSON object containing all input states. Fields are optional
(defaulting to 0/false) but sending the full state is recommended.

```json
{
  "thrust": 1.0,
  "turn": 0.0,
  "shoot": true
}
```

**Field Definitions:**

- `thrust` (f32): Throttle input in `[-1.0, 1.0]` (Godot `Input.get_axis`).
- `turn` (f32): Turn input in `[-1.0, 1.0]`.
- `shoot` (bool): Fire (hold-to-shoot).
  - Client should send `true` while the button is held (`Input.is_action_pressed("shoot")`).
  - The server fires whenever the per-player cooldown allows.

### Transmission Strategy

- **Frequency**: Inputs should be sent at a fixed rate (e.g., 60 times per second).
- **Authority**: The client is a "dumb terminal". It only reports what the player
  _wants_ to do (control intent). The server translates these controls into actual
  movement based on ship physics. The client does not move the player locally
  until the server confirms the movement in a world update.

## 3. Server Messages

The server sends messages wrapped in a JSON object with a `type` and `data` field.

### A. Identity (On Join)

Sent immediately after connection to assign the player's unique ID.

```json
{
  "type": "Identity",

  "data": {
    "player_id": 1234567890
  }
}
```

### B. World Update (Every Tick)

Contains the snapshot of all entities.

```json
{
  "type": "WorldUpdate",

  "data": {
    "tick": 123,

    "entities": [
      {
        "id": 1234567890,

        "x": 100.0,

        "y": 50.0,

        "rot": 1.57
      }
    ],

    "projectiles": [
      {
        "id": 1,
        "owner_id": 1234567890,
        "x": 110.0,
        "y": 40.0,
        "rot": 1.57
      }
    ]
  }
}
```

### C. Game State (On Change)

High-level match state (Lobby, MatchStarting, etc).

```json
{
  "type": "GameState",

  "data": {
    "MatchStarting": { "in_seconds": 3 }
  }
}
```

or

```json
{
  "type": "GameState",

  "data": "MatchRunning"
}
```

### Processing Strategy

1. **Parse JSON**: Check the `type` field.

2. **Identity**: Store `player_id` locally as `my_player_id`.

3. **WorldUpdate**:
   - Iterate through `entities`.

   - If `id == my_player_id`, update local interpolation target (do NOT predict
     movement yet for simplicity).

   - If `id` is new, spawn `player.tscn`.

   - Iterate through `projectiles` (may be missing; treat as empty).
     - If `id` is new, spawn `projectile.tscn`.
     - Snap on the first update to avoid lerping from the default spawn position
       (often `(0, 0)`), then lerp normally.

4. **GameState**: Update UI (timers, screens).
