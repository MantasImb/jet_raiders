# Client Networking

This document describes the current networking behavior of the Godot client.
It reflects the implementation in:

- `Scripts/NetworkManager.gd`
- `Scripts/WorldSync.gd`
- `Scripts/AuthContext.gd`
- `Scripts/auth/AuthStateMachine.gd`
- `Scripts/auth/states/BootstrapState.gd`
- `Scripts/auth/states/GuestIdentityState.gd`
- `Scripts/auth/states/LoginState.gd`
- `Scripts/auth/states/AuthenticatedState.gd`
- `Scripts/PlayerInput.gd`
- `Scripts/Player.gd`
- `Scripts/Projectile.gd`

## Overview

The client uses:

- HTTP (head server) for guest identity and login.
- WebSocket (game server) for realtime gameplay messages.
- A node-based auth FSM for profile load, guest init, login, and retry flow.

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

1. `AuthStateMachine` enters `BootstrapState`.
2. `BootstrapState` immediately transitions to `GuestIdentityState`.
3. `GuestIdentityState` loads `user://guest_profile.json` through
   `AuthContext`.
4. If a valid stored `guest_id` exists, the FSM transitions to `LoginState`.
5. If no valid `guest_id` exists, `GuestIdentityState` calls
   `POST /guest/init`, persists the resolved `guest_id`, and then transitions
   to `LoginState`.
6. `LoginState` enters substate `IDLE_READY` and immediately starts login for
   the current guest auth flow.
7. `LoginState` calls `POST /guest/login`, trims `session_token`, and only
   stores it in `AuthContext.auth_token` if the token is non-blank.
8. On success, the FSM transitions to `AuthenticatedState`.
9. `AuthenticatedState` emits `authenticated(session_token)` from
   `AuthStateMachine` only when `auth_token` is still valid; otherwise it
   recovers by transitioning back to `LoginState`.
10. `NetworkManager` opens a WebSocket connection to the game server.
11. After socket open, `NetworkManager` sends a `Join` message containing the
    auth `session_token`.
12. Server responds with `Identity` and then periodic `WorldUpdate` messages.
13. `NetworkManager` routes each `WorldUpdate` to `WorldSync`.

During `_process`, `NetworkManager` calls `game_socket.poll()` and drains all
available packets.

Auth gating:

- `NetworkManager` only starts the WebSocket flow after
  `AuthStateMachine.is_authenticated()` reports success.
- Reconnect attempts are also gated on authenticated auth state.
- `Join` is only sent after socket open and only if `AuthContext.auth_token` is
  present.

## World Sync

World snapshot application is implemented as a separate node under `Network`:

```text
Network
├── AuthContext
├── AuthStateMachine
├── WorldSync
├── NetworkUI
└── SpawnedNodes
```

`NetworkManager.gd` is attached to the `Network` node itself; it is not a
separate child node in the scene tree.

Responsibilities:

- `NetworkManager`: transport, message parsing, auth-gated join, reconnect
- `WorldSync`: spawn, update, and despawn scene nodes from `WorldUpdate`
- `SpawnedNodes`: parent node for synchronized player/projectile instances

This separation allows future transports such as UDP to reuse the same world
application logic without duplicating scene sync behavior.

## Auth State Machine

The auth flow is implemented as a scene-tree FSM under `Network`:

```text
Network
├── AuthContext
├── AuthStateMachine
│   ├── BootstrapState
│   ├── GuestIdentityState
│   ├── LoginState
│   └── AuthenticatedState
├── WorldSync
├── NetworkUI
└── SpawnedNodes
```

`NetworkManager.gd` is attached to the `Network` node that owns this subtree.

Top-level state meanings:

- `BootstrapState`: resets auth runtime state and enters guest identity flow.
- `GuestIdentityState`: loads profile, validates `guest_id`, requests
  `/guest/init` when needed, and retries with backoff.
- `LoginState`: handles `login_requested`, calls `/guest/login`, and retries
  with backoff. In the current guest flow it auto-starts login on entry and
  only accepts a non-blank trimmed `session_token`.
- `AuthenticatedState`: emits the success signal used by `NetworkManager`, or
  returns to `LoginState` if entered without a valid `auth_token`.

Important `GuestIdentityState` substates:

- `LOAD_PROFILE`
- `REQUEST_GUEST_ID`
- `RETRY_WAIT`
- `FAILED`
- `READY`

Important `LoginState` substates:

- `IDLE_READY`
- `REQUEST_LOGIN`
- `RETRY_WAIT`
- `FAILED_TERMINAL`
- `SUCCESS`

Retry behavior:

- Guest init retries and login retries are tracked separately.
- Backoff is exponential from `0.5s` up to `6.0s`.
- Retry scheduling is owned by `AuthStateMachine`.
- `GuestIdentityState` stops in `FAILED` after exhausting retries.
- `LoginState` stops in `FAILED_TERMINAL` after exhausting retries.
- Pressing the login button while `LoginState` is in `FAILED_TERMINAL`
  calls `AuthStateMachine.request_login()` and restarts login manually.

Runtime identity note:

- `auth_token` is produced by the HTTP auth flow and used in `Join`.
- `local_player_id` is produced later by the game server `Identity` message.
- A client can therefore be authenticated before it knows which spawned player
  node is locally owned.

Failure path note:

- If guest init exhausts retries, auth stops in `GuestIdentityState.FAILED`.
- If login exhausts retries, auth stops in `LoginState.FAILED_TERMINAL`.
- In either failure state, networking does not start until the player retries
  via `AuthStateMachine.request_login()`.

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

- Stores `data.player_id` in `auth_context.local_player_id`.

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
  - `WorldSync` spawns missing players under `Network/SpawnedNodes`.
  - Update existing players via `Player.update_state(...)`.
  - Despawn players missing from the latest snapshot.
- Projectiles:
  - `WorldSync` spawns missing projectile nodes named `proj_<id>`.
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
- Guard: avoids reconnect attempts while already `CONNECTING`,
  `CONNECTED`, or `RETRY_WAIT`.
- On successful reconnect: reset reconnect counters and send `Join` again.

`NetworkManager` tracks socket lifecycle with an internal enum:

- `IDLE`
- `CONNECTING`
- `CONNECTED`
- `RETRY_WAIT`

`PlayerInput` uses `NetworkManager.has_open_connection()` instead of reading
connection flags directly.

## Notes and Constraints

- `AuthContext` owns auth data, profile persistence, and validation helpers.
- `AuthStateMachine` is the source of truth for auth lifecycle state.
- `NetworkManager` no longer starts auth; it only reacts to auth success and
  manages the WebSocket lifecycle.
- `WorldSync` owns scene mutation for synchronized players and projectiles.
- Network UI is hidden on successful socket connection and shown on close.
- On server close, all spawned networked entities are cleared.
- Gameplay state is server-authoritative; client-side movement is smoothing
  only, not authoritative simulation.
