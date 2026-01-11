# Clean Architecture Separation Appendix (Extra detail)

This appendix contains extra examples, checklists, and reference snippets pulled
from the longer separation docs.

- For rules/boundaries: `CLEAN_ARCHITECTURE_GUIDELINES.md`
- For the simple action plan: `CLEAN_ARCHITECTURE_SEPARATION_GUIDE.md`

## Mental model (3 roles)

- **Game brain (domain):** `state.rs`, `systems/*`, and the domain logic used by
  `game.rs`.
- **Messenger (adapter):** `net.rs` and the message shapes in `protocol.rs`.
- **Glue (bootstrap):** `main.rs`.

If you’re unsure where code goes:

- Changes the world → domain (`game.rs` / `systems/*` / `state.rs`).
- Talks JSON/WebSockets → adapter (`protocol.rs` + `net.rs`).
- Creates channels/spawns tasks/starts server → glue (`main.rs`).

## Single-world wiring reference (pipeline)

Target pipeline:

- Many WS clients send inputs into one channel: `tx_input`.
- `game.rs` drains `rx_input` and updates the world.
- `game.rs` emits snapshots/events into `tx_out`.
- Each client receives from `rx_out` and writes to its socket.

Note: a single `mpsc::Receiver` cannot be cloned for many connections; use
`tokio::sync::broadcast` or per-connection receivers for fan-out.

## Reference snippets (shapes only)

These are intentionally partial "shapes" to keep the intent clear.

### `protocol.rs` (wire DTOs)

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerInput {
    pub thrust: bool,
    pub turn: f32,
    pub shoot: bool,
}
```

### `state.rs` (domain model)

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
    pub input: PlayerInputState,
}

pub struct PlayerInputState {
    pub thrust: bool,
    pub turn: f32,
    pub shoot: bool,
}
```

Key point: don’t reuse `protocol::PlayerInput` inside `state.rs`; convert at the
boundary (usually in `net.rs`).

### `systems/ship_movement.rs` (pure-ish updates)

```rust
use crate::state::GameState;

pub fn update(state: &mut GameState, dt: f32) {
    for player in state.players.values_mut() {
        player.rot += player.input.turn * dt;
        if player.input.thrust {
            player.x += player.rot.cos() * 100.0 * dt;
            player.y += player.rot.sin() * 100.0 * dt;
        }
    }
}
```

### `game.rs` (tick loop shape)

```rust
use tokio::sync::mpsc;
use tokio::time::{sleep_until, Duration, Instant};

use crate::protocol;
use crate::state::GameState;

pub struct DomainInput {
    pub player_id: u64,
    pub thrust: bool,
    pub turn: f32,
    pub shoot: bool,
}

pub async fn run_game_loop(
    mut rx_input: mpsc::Receiver<DomainInput>,
    tx_out: mpsc::Sender<protocol::ServerMessage>,
    tick_rate: f32,
) {
    let mut state = GameState { players: Default::default(), projectiles: vec![] };
    let tick = Duration::from_secs_f32(1.0 / tick_rate);

    loop {
        let frame_start = Instant::now();

        while let Ok(input) = rx_input.try_recv() {
            apply_input(&mut state, input);
        }

        let dt = tick.as_secs_f32();
        crate::systems::ship_movement::update(&mut state, dt);
        crate::systems::projectiles::update(&mut state, dt);

        let msg = protocol::ServerMessage::WorldSnapshot(to_snapshot(&state));
        if tx_out.send(msg).await.is_err() {
            break;
        }

        sleep_until(frame_start + tick).await;
    }
}
```

Note: stricter separation is to emit domain snapshots/events and convert to
`protocol::ServerMessage` in `net.rs`, but emitting protocol outputs can be OK
early if the domain stays free of Axum/WebSocket types.

## Example end-state shapes

### `main.rs` (composition only)

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let config = config::Config::from_env()?;

    let lobby_manager = lobby::LobbyManager::new(config.clone());
    let app = net::router(config, lobby_manager);

    axum::Server::bind(&config.bind_addr())
        .serve(app.into_make_service())
        .await?;
    Ok(())
}
```

### `net.rs` (router shape)

```rust
pub fn router(config: Config, lobby_manager: LobbyManager) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state((config, lobby_manager))
}
```

### `lobby.rs` (spawning per lobby)

```rust
pub struct LobbyManager {
    lobbies: DashMap<String, LobbyHandle>,
    config: Config,
}

impl LobbyManager {
    pub async fn join_or_create(&self, lobby_id: &str) -> LobbyHandle {
        self.lobbies
            .entry(lobby_id.to_string())
            .or_insert_with(|| self.spawn_lobby(lobby_id))
            .clone()
    }
}
```

## Done checklists

### Separation done (single-world)

You’re separated correctly when:

- `main.rs` has no `GameState` mutations.
- `main.rs` has no JSON encode/decode logic.
- `net.rs` has no calls to `ship_movement::update` or `projectiles::update`.
- `systems/*` do not import `axum`, `tokio::net`, or `serde_json`.
- `state.rs` does not import `axum` or `serde`.
- `protocol.rs` does not import `GameState`.

### Wiring checklist

- Does `main.rs` only compose components and start the server?
- Do systems depend only on domain types, never on networking or tasks?
- Is the protocol layer the only place that touches serde and message schemas?
- Are lobby and game loop ownership/lifetimes clear, with channels passed in
  from the bootstrap layer?

## Common mistakes (quick fixes)

- Protocol DTOs used as domain state
  - Fix: keep a domain input/state type and convert in `net.rs` (or boundary
    code in `game.rs`).

- Game loop knows about sockets/framework
  - Fix: move socket code to `net.rs`; keep channels/time/domain in `game.rs`.

- Broadcasting with a single `mpsc::Receiver`
  - Fix: use `broadcast` for outgoing messages or per-connection channels.

## Suggested refactor order (for minimal pain)

1. Extract `protocol.rs` (compile).
2. Extract `state.rs` (compile).
3. Extract `systems/*` (compile).
4. Extract `game.rs` and spawn it from `main.rs` (compile).
5. Extract `net.rs` and make `main.rs` call `net::router(...)` (compile).
