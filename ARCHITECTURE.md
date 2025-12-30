# Jet Raiders - Multiplayer Architecture Plan

This document outlines the architecture for converting the "Jet Raiders" Godot game
from a peer-to-peer/host-based model to a robust Client-Server Authoritative
model using Rust.

## 1. High-Level Architecture

We are moving to a **Server-Authoritative** model. The server is the single
source of truth. Clients are "dumb" terminals that send inputs and render the
state received from the server.

```mermaid
graph TD
    Client[Godot Web Client] <-->|WebSocket (JSON)| Axum[Rust Server (Axum)]
    Axum -->|Connection/Disconnection| GameLoop
    Axum -->|Player Inputs| InputQueue

    subgraph Rust Server
        GameLoop[Game Loop (60 Hz)]
        InputQueue[Input Queue]
        State[Game World State]

        GameLoop -->|Read| InputQueue
        GameLoop -->|Update| State
        GameLoop -->|Snapshot| State
    end

    State -->|World Snapshot| Axum
```

## 1.1 Multi-Lobby Architecture

The server acts as a **Lobby Manager**.

1. **Connection**: Client connects to `ws://server/ws`.
2. **Handshake**: Client sends `Join { lobby_id: "Room1" }`.
3. **Routing**:
    - If "Room1" exists: Connect client to that running game loop.
    - If "Room1" is missing: Create a new `GameLoop` task for it.
4. **Isolation**: Each Lobby runs in its own Tokio Task. A crash or lag spike
    in one lobby does not affect others.

## 2. Server Project Structure (`server/src/`)

We will use a modular "Simple Structs" approach. We will implement **Multiple
Lobbies** support, where the main server manages multiple isolated game
sessions.

```text
server/
├── Cargo.toml          # Dependencies: axum, tokio, serde, etc.
└── src/
    ├── main.rs         # Entry point. Sets up Axum and the LobbyManager.
    ├── config.rs       # Shared constants.
    ├── protocol.rs     # The "Language". Shared Structs/Enums.
    ├── net.rs          # WebSocket handling. Handshakes & routing to lobbies.
    ├── lobby.rs        # Manages active game sessions (HashMap<ID, Channel>).
    ├── game.rs         # The Game Loop (One instance per Lobby).
    ├── state.rs        # Data definitions: GameState, Player, Projectile.
    └── systems/        # Logic Modules.
        ├── mod.rs
        ├── movement.rs
        └── combat.rs
```

## 3. Communication Protocol (`protocol.rs`)

We will use JSON for messages initially for easy debugging.

### Client -> Server (`ClientMessage`)

Sent by the Godot client to the Rust server.

```rust
enum ClientMessage {
    // Initial handshake to join a specific room
    Join { lobby_id: String, username: String },

    // Sent every frame/tick by the client
    Input {
        thrust: bool,       // W / Up
        turn: f32,          // -1.0 (Left) to 1.0 (Right)
        shoot: bool,        // Space
    },

    // Heartbeat to keep connection alive
    Ping,
}
```

### Server -> Client (`ServerMessage`)

Sent by the Rust server to the Godot client.

```rust
enum ServerMessage {
    // The main sync message. Sent every tick (or every X ticks).
    WorldSnapshot {
        players: Vec<PlayerData>,
        projectiles: Vec<ProjectileData>,
        server_time: f64,
    },

    // Specific events that might trigger sounds or visual effects
    GameEvent(GameEvent),

    Pong,
}

enum GameEvent {
    PlayerJoined { id: u64, name: String },
    PlayerLeft { id: u64 },
    PlayerDied { victim_id: u64, killer_id: u64 },
}
```

## 4. Game State Data (`state.rs`)

The `GameState` struct holds the entire world.

```rust
pub struct GameState {
    pub players: HashMap<u64, Player>,
    pub projectiles: Vec<Projectile>,
    pub map_width: f32,
    pub map_height: f32,
}

pub struct Player {
    pub id: u64,
    pub name: String,
    pub position: Vec2, // x, y
    pub rotation: f32,  // radians
    pub velocity: Vec2,
    pub health: f32,
    pub score: i32,
    pub input: PlayerInput, // Last received input
}

pub struct Projectile {
    pub id: u64,
    pub owner_id: u64,
    pub position: Vec2,
    pub velocity: Vec2,
    pub life_time: f32, // Remaining time to live
}
```

## 5. The Game Loop (`game.rs`)

The server runs at a fixed tick rate (e.g., 60 ticks per second).

**Loop Cycle:**

1. **Sleep**: Wait until the next tick time (ensures consistent speed).
2. **Process Inputs**: Drain the `InputQueue`. Update `Player.input` for each client.
3. **Run Systems**:
   - `movement::update(&mut state)`: Apply thrust, update positions, wrap
     around map borders.
   - `combat::update(&mut state)`: Move projectiles, check collisions (AABB or
     Circle), apply damage, handle respawns.
4. **Broadcast State**: Serialize `GameState` into a `WorldSnapshot` and
   send it to all connected clients via `net.rs`.

## 6. Client Refactoring (Godot)

The Godot client needs to change from "Processing Logic" to "Displaying State".

1. **NetworkManager.gd**:
   - Replace `ENetMultiplayerPeer` with `WebSocketPeer` (or a dedicated WebSocket
     node).
   - Implement a buffer to handle incoming JSON packets.
   - Deserialization logic: `JSON -> Dictionary -> Update Nodes`.

2. **Player.gd**:
   - Remove physics processing (`_physics_process`). The client no longer
     calculates position!
   - Add `target_position` and `target_rotation`.
   - Use `lerp` (Linear Interpolation) in `_process` to smoothly move the
     visual sprite to the `target_position` received from the server. This
     hides network latency (Smoothing).

3. **GameManager.gd**:
   - Remove game logic (score checking, respawning).
   - Listen for `GameEvent` messages from the server to show UI (e.g., "Player X
     won!").

## 7. Future Extensibility

- **New Weapons**: Add a `weapon_type` enum to `Player` and switch logic in `combat.rs`.
- **Power-ups**: Add a `Vec<PowerUp>` to `GameState` and a `powerup.rs` system.
- **Binary Protocol**: Switch `serde_json` to `bincode` in `net.rs` for smaller
  packets (better performance) without changing game logic.
