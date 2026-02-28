# Client Networking

This document describes the current networking behavior of the Godot client.
It reflects the implementation in:

- `Scripts/NetworkManager.gd`
- `Scripts/UserManager.gd`
- `Scripts/PlayerInput.gd`
- `Scripts/Player.gd`
- `Scripts/Projectile.gd`

## Overview

The client uses:

- HTTP (head server) for guest identity and login.
- WebSocket (game server) for realtime gameplay messages.

Transport message format is JSON with a top-level wrapper:

```json
{
  "type": "MessageType",
  "data": {}
}
```

## Endpoints (Current Local/Test Setup)

- Head server base URL: `http://127.0.0.1:3000`
- Game server WebSocket URL (test mode): `ws://127.0.0.1:3001/ws`

`NetworkManager` only auto-connects to the game server in test mode
(`GameManager.TEST_MODE == true`).

## Connection Lifecycle

1. `UserManager` resolves or creates `guest_id` via:
   - `POST /guest/init` (first run or invalid stored ID)
   - local profile load (`user://guest_profile.json`)
2. `UserManager` logs in via `POST /guest/login` and stores `session_token`.
3. `UserManager` emits `authenticated(session_token)`.
4. `NetworkManager` opens a WebSocket connection to the game server.
5. After socket open, `NetworkManager` sends a `Join` message containing the
   auth `session_token`.
6. Server responds with `Identity` and then periodic `WorldUpdate` messages.

During `_process`, `NetworkManager` calls `game_socket.poll()` and drains all
available packets.

## Client -> Server Messages

### Join

Sent once after the socket opens (when authenticated).

```json
{
  "type": "Join",
  "data": {
    "session_token": "..."
  }
}
```

### Input

Sent by `PlayerInput` from the local-owned `Player` instance only.

```json
{
  "type": "Input",
  "data": {
    "thrust": 1.0,
    "turn": 0.0,
    "shoot": true
  }
}
```

Input fields:

- `thrust` (`float`): from `Input.get_axis("throttle_down", "throttle_up")`
- `turn` (`float`): from `Input.get_axis("turn_left", "turn_right")`
- `shoot` (`bool`): from `Input.is_action_pressed("shoot")`

`PlayerInput` sends every physics frame while connected. Authority remains
server-side; the client submits intent only.

## Server -> Client Messages

### Identity

```json
{
  "type": "Identity",
  "data": {
    "player_id": "1234567890"
  }
}
```

Client behavior:

- Stores `data.player_id` in `user.local_player_id`.

### WorldUpdate

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

Client behavior:

- Players:
  - Spawn missing players under `Network/SpawnedNodes`.
  - Update existing players via `Player.update_state(...)`.
  - Despawn players missing from the latest snapshot.
- Projectiles:
  - Spawn missing projectile nodes named `proj_<id>`.
  - Update existing projectiles via `Projectile.update_state(...)`.
  - Despawn projectiles missing from the latest snapshot.

Interpolation behavior:

- `Player` lerps toward target transform (`smoothing_speed = 15.0`).
- If player distance to target exceeds `50`, it snaps to target position.
- `Projectile` lerps toward target transform (`smoothing_speed = 25.0`).
- New projectile snaps on first update to avoid lerping from `(0, 0)`.

### GameState

```json
{
  "type": "GameState",
  "data": "MatchRunning"
}
```

or:

```json
{
  "type": "GameState",
  "data": {
    "MatchStarting": {
      "in_seconds": 3
    }
  }
}
```

Current behavior: logged by `NetworkManager`. UI/game-state wiring is not
implemented in this file yet.

## Reconnect Behavior (Test Mode)

Reconnect logic is enabled only in test mode.

- Trigger: connection failure or closed socket.
- Conditions: only when user is authenticated.
- Backoff: exponential from `0.5s` up to `6.0s`.
- Guard: avoids reconnect attempts while already connecting/open.
- On successful reconnect: reset reconnect counters and send `Join` again.

## Notes and Constraints

- Network UI is hidden on successful socket connection and shown on close.
- On server close, all spawned networked entities are cleared.
- Gameplay state is server-authoritative; client-side movement is smoothing
  only, not authoritative simulation.
