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
3. **Polling**: The client must call `socket.poll()` every frame (in `_process`)
   to process incoming packets and maintain the connection.

## 2. Sending Inputs

Inputs are sent from the client to the server to drive the simulation.

### Input Packet Structure

The server expects a JSON object containing all input states. Fields are optional
(defaulting to 0/false) but sending the full state is recommended.

```json
{
  "thrust": 1,
  "turn": 0,
  "shoot": true,
  "special": false
}
```

**Field Definitions:**

- `thrust` (i8): Throttle/Brake control
  - `1` = Throttle (accelerate forward)
  - `0` = Coast (no acceleration)
  - `-1` = Brake (reverse/slow down)
- `turn` (i8): Turning control
  - `1` = Turn right
  - `0` = Straight
  - `-1` = Turn left
- `shoot` (bool): Primary weapon (shoot)
- `special` (bool): Special attack/ability

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

4. **GameState**: Update UI (timers, screens).

## 5. Migration Note

The client is currently transitioning from Godot's built-in
`ENetMultiplayerPeer` to this WebSocket-based architecture. See
`CLIENT_MIGRATION.md` for the step-by-step cleanup and reconstruction plan.
