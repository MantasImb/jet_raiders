# Jet Raiders - Client Migration Plan

This document outlines the steps to refactor the Godot client from a P2P/Host
model to a Server-Authoritative "dumb terminal". The goal is to strip existing
multiplayer logic and rebuild it to interface with the Rust server.

## Phase 1: The "Lobotomy" (Cleanup)

We must remove all logic where the client currently acts as the server (hosting,
logic processing, collision handling).

### 1. `NetworkManager.gd`
- [ ] Remove `ENetMultiplayerPeer` code.
- [ ] Remove `create_server()` and `create_client()` logic related to ENet.
- [ ] Remove existing `rpc` functions (`_on_player_connected`, etc.).

### 2. `GameManager.gd`
- [ ] Remove game state logic (score tracking, win conditions).
- [ ] Remove `check_win_condition` or similar RPCs.
- [ ] **Goal**: It should only be responsible for updating the UI (Scoreboard,
  Game Over screen) based on signals/events.

### 3. `Player.gd`
- [ ] Remove `_physics_process`: The client should NEVER calculate physics or
  collisions.
- [ ] Remove `take_damage()` logic: Health is just a number sent by the server.
- [ ] Remove `die()` logic: Death is an event sent by the server.
- [ ] **Keep**: `_process` (for visual interpolation/smoothing) and FX logic
  (particle emission, sound playback).

### 4. `Projectile.gd`
- [ ] Remove `_physics_process`.
- [ ] Remove collision detection (`_on_body_entered`).
- [ ] **Goal**: It becomes a visual-only object that moves from A to B or
  exists at a position given by the server.

---

## Phase 2: The New Network Layer

### 1. WebSocket Client
- [ ] Create a new `WebSocketClient` class (or update `NetworkManager`).
- [ ] Implement connection to `ws://127.0.0.1:3000/ws`.
- [ ] Implement a **Message Loop** (`_process` or `_physics_process`) to poll
  `socket.get_ready_state()` and `socket.get_data()`.

### 2. Protocol Implementation
- [ ] Create a helper to serialize `ClientMessage` (Input, Login) to JSON.
- [ ] Create a helper to deserialize `ServerMessage` (Snapshot, Events) from
  JSON.

---

## Phase 3: Visual Reconstruction

### 1. State Replication
- [ ] In `NetworkManager`, handle the `WorldSnapshot` message.
- [ ] Iterate through `players` data.
    - If Player ID exists: Update `target_position`, `target_rotation`.
    - If Player ID is new: Spawn `player.tscn`.
    - If Player ID is missing: `queue_free()` the node.

### 2. Interpolation (Smoothing)
- [ ] Update `Player.gd` to strictly `lerp` between its current position and
  the received `target_position`.
- [ ] **Do not** apply velocity directly.

---

## Phase 4: Input Handling

### 1. Input Collection
- [ ] In `_process` (or a dedicated Input script), collect:
    - `Input.is_action_pressed("thrust")`
    - `Input.is_action_pressed("shoot")`
    - Mouse position / Rotation.

### 2. Transmission
- [ ] Construct the `Input` JSON packet.
- [ ] Send it via WebSocket at a fixed rate (e.g., 60 times/sec or every frame).
