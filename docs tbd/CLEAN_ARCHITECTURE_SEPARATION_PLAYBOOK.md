# Clean Architecture Separation Playbook (Single World)

> Note: For the authoritative "where does this logic go?" rules, see
> `CLEAN_ARCHITECTURE_GUIDELINES.md`.

This is a step-by-step, easy-to-follow playbook for moving logic out of
`server/src/main.rs` and into the files described in `ARCHITECTURE.md`.

The goal: **`main.rs` becomes a thin bootstrap file** and your server follows the clean
architecture dependency rule:

- Outer layers (Axum/web, runtime wiring) can depend on inner layers.
- Inner layers (game rules and state) must **not** depend on outer layers.

At this stage, you are building a **single global world task** (no lobbies yet).

## 0) The mental model (dumbed down)

Think of your server as three parts:

1. **The game brain** (domain): "What is the world? How does it update?"
   - Files: `state.rs`, `systems/*`, and the core logic inside `game.rs`.

2. **The messenger** (network adapter): "How do messages go in/out over WebSockets?"
   - Files: `net.rs`, and message definitions in `protocol.rs`.

3. **The glue** (bootstrap): "Start the program, wire the brain and messenger together."
   - File: `main.rs`.

If you ever wonder where code should go:

- If it **changes the world** → domain (`game.rs` / `systems/*` / `state.rs`).
- If it **talks JSON/WebSockets** → adapter (`protocol.rs` + `net.rs`).
- If it **creates channels / spawns tasks / starts Axum** → glue (`main.rs`).

## 1) Target file responsibilities (single-world)

Use this table as your north star:

| File | Owns | Must NOT own |
|------|------|--------------|
| `main.rs` | config/tracing init, creating channels, spawning the world task, building router, starting server | websocket message parsing, game rules, collision/movement, lobby maps |
| `net.rs` | Axum router, WS upgrade, read/write loop for sockets, translating protocol messages to domain inputs | world update logic, movement/combat calculations |
| `protocol.rs` | `ClientMessage`, `ServerMessage`, `GameEvent` (wire DTOs), serde derives | `GameState` internals, physics rules |
| `state.rs` | `GameState`, `Player`, `Projectile`, math helpers/types | serde/wire DTOs, Axum types |
| `game.rs` | the world task: tick loop, draining inputs, calling systems, producing snapshots/events | Axum router, socket types, JSON parsing |
| `systems/*` | pure-ish update functions that mutate `GameState` (`movement`, `combat`, etc.) | networking, channels, serde |

## 2) The single-world wiring you are aiming for

You want a very simple pipeline:

- Many WebSocket clients send **inputs** into one channel: `tx_input`.
- The game loop consumes inputs from `rx_input` and updates the world.
- The game loop produces **snapshots/events** into `tx_out`.
- Each connected client gets snapshots/events from `rx_out` and writes them to its socket.

> Note: a plain `mpsc::Receiver` can’t be cloned for multiple clients. When you get there, you
> typically switch to `broadcast` or manage per-connection receivers (e.g. `broadcast::Sender`).
> For the separation work, the exact channel type is less important than the boundaries.

## 3) Step-by-step: how to disassemble `main.rs`

### Step 1: Make a list of what `main.rs` currently does

Open `server/src/main.rs` and categorize each chunk into one of these buckets:

- **Bootstrap**: tracing/config, bind address, creating routers, spawning tasks.
- **Networking**: WebSocket handler, `while let Some(msg)`, JSON decode/encode.
- **Domain**: movement/combat updates, collision checks, score/health changes.

Write down the list (even in your head) before moving code.

### Step 2: Create/confirm the modules exist

Make sure these files exist under `server/src/`:

- `config.rs`
- `protocol.rs`
- `net.rs`
- `state.rs`
- `game.rs`
- `systems/mod.rs`, `systems/movement.rs`, `systems/combat.rs`

If a file doesn’t exist yet, create it with a minimal stub so you can move code gradually.

### Step 3: Move message enums into `protocol.rs`

Move all wire-level message definitions out of `main.rs`.

**In `protocol.rs`:**

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Input(PlayerInput),
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    WorldSnapshot(WorldSnapshot),
    GameEvent(GameEvent),
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GameEvent {
    PlayerJoined { id: u64, name: String },
    PlayerLeft { id: u64 },
}

// Keep these as protocol DTOs (shapes meant for the network).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInput {
    pub thrust: bool,
    pub turn: f32,
    pub shoot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    pub server_time: f64,
    pub players: Vec<PlayerData>,
    pub projectiles: Vec<ProjectileData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerData {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    pub health: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectileData {
    pub id: u64,
    pub x: f32,
    pub y: f32,
}
```

**Why:** these are the contract between server and client. They belong in the protocol layer.

### Step 4: Move the world structs into `state.rs`

Put your authoritative world model in `state.rs`.

**In `state.rs`:**

```rust
use std::collections::HashMap;

pub struct GameState {
    pub players: HashMap<u64, Player>,
    pub projectiles: Vec<Projectile>,
}

pub struct Player {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    pub health: f32,

    // Store last input (authoritative input state).
    pub input: PlayerInputState,
}

pub struct Projectile {
    pub id: u64,
    pub owner_id: u64,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub ttl: f32,
}

pub struct PlayerInputState {
    pub thrust: bool,
    pub turn: f32,
    pub shoot: bool,
}
```

**Important:** `state.rs` should not depend on `axum`, `WebSocket`, or serde/wire DTOs.

> If you currently reuse `protocol::PlayerInput` inside `state.rs`, that’s a sign you’re mixing
> wire DTOs with domain. Prefer a separate `PlayerInputState` in domain and convert in `net.rs`.

### Step 5: Make `systems/*` pure update functions

Your systems should be simple functions that mutate `GameState`.

**In `systems/movement.rs`:**

```rust
use crate::state::GameState;

pub fn update(state: &mut GameState, dt: f32) {
    for player in state.players.values_mut() {
        // Example: apply turning and thrust.
        player.rot += player.input.turn * dt;
        if player.input.thrust {
            player.x += player.rot.cos() * 100.0 * dt;
            player.y += player.rot.sin() * 100.0 * dt;
        }
    }
}
```

**Rule:** systems should not create tasks, read sockets, serialize JSON, or know about channels.

### Step 6: Move the tick loop into `game.rs`

Your game loop is the world task. It owns `GameState` and calls systems.

**In `game.rs` (shape):**

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep_until, Duration, Instant};

use crate::protocol;
use crate::state::GameState;

pub async fn run_game_loop(
    mut rx_input: mpsc::Receiver<DomainInput>,
    tx_out: mpsc::Sender<protocol::ServerMessage>,
    tick_rate: f32,
) {
    let mut state = GameState { players: Default::default(), projectiles: vec![] };
    let tick = Duration::from_secs_f32(1.0 / tick_rate);

    loop {
        let frame_start = Instant::now();

        // 1) Drain inputs
        while let Ok(input) = rx_input.try_recv() {
            apply_input(&mut state, input);
        }

        // 2) Update world
        let dt = tick.as_secs_f32();
        crate::systems::movement::update(&mut state, dt);
        crate::systems::combat::update(&mut state, dt);

        // 3) Emit snapshot
        let msg = protocol::ServerMessage::WorldSnapshot(to_snapshot(&state));
        if tx_out.send(msg).await.is_err() {
            break;
        }

        // 4) Sleep until next tick
        sleep_until(frame_start + tick).await;
    }
}

// Domain input type (separate from protocol wire input).
pub struct DomainInput {
    pub player_id: u64,
    pub thrust: bool,
    pub turn: f32,
    pub shoot: bool,
}

fn apply_input(state: &mut GameState, input: DomainInput) {
    if let Some(p) = state.players.get_mut(&input.player_id) {
        p.input.thrust = input.thrust;
        p.input.turn = input.turn;
        p.input.shoot = input.shoot;
    }
}

fn to_snapshot(state: &GameState) -> protocol::WorldSnapshot {
    protocol::WorldSnapshot {
        server_time: 0.0,
        players: state
            .players
            .values()
            .map(|p| protocol::PlayerData {
                id: p.id,
                x: p.x,
                y: p.y,
                rot: p.rot,
                health: p.health,
            })
            .collect(),
        projectiles: state
            .projectiles
            .iter()
            .map(|pr| protocol::ProjectileData {
                id: pr.id,
                x: pr.x,
                y: pr.y,
            })
            .collect(),
    }
}
```

**The key separation idea:** `game.rs` receives **domain inputs** and emits **protocol outputs**.
If you want to be stricter, you can emit domain snapshots and convert in `net.rs`, but emitting
protocol snapshots is acceptable early on if it keeps the core free of Axum/WebSocket types.

### Step 7: Move all WebSocket code into `net.rs`

`net.rs` owns:

- the Axum route (`/ws`)
- upgrade to WebSocket
- per-connection read/write loops
- translation: `protocol::ClientMessage` → `game::DomainInput`

**Shape (pseudocode):**

```rust
pub struct AppState {
    pub tx_input: mpsc::Sender<game::DomainInput>,
    pub tx_out: broadcast::Sender<protocol::ServerMessage>,
}

pub fn router(state: AppState) -> Router {
    Router::new().route("/ws", get(ws_handler)).with_state(state)
}
```

When you implement it, keep these rules:

- No movement/combat code in `net.rs`.
- No socket/serde code in `game.rs`.

### Step 8: Reduce `main.rs` to wiring only

Once the logic is moved out, `main.rs` should look like this (conceptually):

1. init tracing/config
2. create channels
3. spawn game loop
4. build router
5. start server

If your `main.rs` contains a `loop { ... }` that updates the world, that code belongs in
`game.rs`.

## 4) “You’re done” checklist

You have achieved the separation when:

- `main.rs` has no `GameState` mutations.
- `main.rs` has no JSON encode/decode logic.
- `net.rs` has no calls to `movement::update` or `combat::update`.
- `systems/*` do not import `axum`, `tokio::net`, or `serde_json`.
- `state.rs` does not import `axum` or `serde`.
- `protocol.rs` does not import `GameState`.

## 5) Common mistakes (and how to fix them)

### Mistake: using protocol DTOs inside domain

If you see `crate::protocol::PlayerInput` inside `state.rs` or `systems/*`:

- Create a domain input type (e.g. `state::PlayerInputState`).
- Convert in `net.rs` or `game.rs` at the boundary.

### Mistake: game loop knows about sockets

If `game.rs` imports Axum WebSocket types:

- Move that code to `net.rs`.
- Replace it with channels in `game.rs`.

### Mistake: “broadcasting” with a single `mpsc::Receiver`

If multiple connections need the same outgoing snapshots/events:

- Use `tokio::sync::broadcast` for outgoing messages.
- Or have `net.rs` create per-connection channels.

## 6) Suggested order of changes (minimal pain)

Follow this order to keep the project compiling during the refactor:

1. Extract `protocol.rs` (compile).
2. Extract `state.rs` (compile).
3. Extract `systems/*` (compile).
4. Extract `game.rs` and spawn it from `main.rs` (compile).
5. Extract `net.rs` and make `main.rs` call `net::router(...)` (compile).

After each step, run:

```bash
cd server
cargo check
```

## 7) What to ask yourself during the refactor

- “Is this code about the rules of the world, or about sending/receiving bytes?”
- “Could this code be reused if I replaced Axum with a different web framework?”
- “Could the game loop run in a unit test without network?”

If the answer is **yes**, it’s probably in the right layer.
