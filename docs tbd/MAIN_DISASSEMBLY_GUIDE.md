# Disassembling `main.rs` with Clean Architecture

> Note: For the authoritative "where does this logic go?" rules, see
> `CLEAN_ARCHITECTURE_GUIDELINES.md`.

Use this guide to split `main.rs` into the modules outlined in `ARCHITECTURE.md` while
keeping clean architecture boundaries.

## Target responsibilities

- `main.rs`: bootstrap only (config, tracing, router/server startup).
- `net.rs`: Axum router and WebSocket handlers; translate wire ↔ domain messages.
- `lobby.rs`: lobby registry, channel wiring, spawning game loops per lobby.
- `game.rs`: game loop task using domain types; runs systems and emits snapshots/events.
- `state.rs` and `systems/`: domain data and pure update logic, no Axum/Tokio details.
- `protocol.rs`: wire DTOs (`ClientMessage`, `ServerMessage`, `GameEvent`) and serde.

## Quick-start: single-world (no lobbies yet)

- In `main.rs`, create one pair of channels (`tx_input`, `rx_input`, `tx_out`, `rx_out`), spawn
  `tokio::spawn(game::run_game_loop(rx_input, tx_out, config.clone()))`, and pass
  `(tx_input, rx_out)` into the router builder.
- In `net.rs`, the WS handler upgrades, reads `ClientMessage::Input` into `tx_input`, and streams
  `ServerMessage` from `rx_out` back to the socket; no lobby map or spawning logic yet.
- Keep domain types in `state.rs` and wire DTOs in `protocol.rs`; the game loop stays framework
  agnostic and only knows about channels and domain types.

## Example end-state: `main.rs`

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

`main` should not contain game rules, serialization, or lobby maps—only composition.

## Networking layer: `net.rs`

```rust
pub fn router(config: Config, lobby_manager: LobbyManager) -> Router {
    Router::new()
        .route("/ws", get(ws_handler))
        .with_state((config, lobby_manager))
}

async fn ws_handler(
    State((config, lobby_manager)): State<(Config, LobbyManager)>,
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, lobby_manager.clone(), addr, config.clone()))
}
```

`handle_socket` receives `ClientMessage`, asks `lobby_manager` for a lobby handle, and
passes channels to the game loop. No game logic here—only transport and translation to/from
`protocol.rs` types.

## Lobby orchestration: `lobby.rs`

```rust
pub struct LobbyManager {
    lobbies: DashMap<String, LobbyHandle>,
    config: Config,
}

impl LobbyManager {
    pub fn new(config: Config) -> Self { /* ... */ }

    pub async fn join_or_create(&self, lobby_id: &str) -> LobbyHandle {
        self.lobbies
            .entry(lobby_id.to_string())
            .or_insert_with(|| self.spawn_lobby(lobby_id))
            .clone()
    }

    fn spawn_lobby(&self, lobby_id: &str) -> LobbyHandle {
        let (tx_input, rx_input) = mpsc::channel(self.config.input_buffer);
        let (tx_out, rx_out) = mpsc::channel(self.config.snapshot_buffer);
        tokio::spawn(game::run_game_loop(rx_input, tx_out, self.config.clone()));
        LobbyHandle { tx_input, rx_out }
    }
}
```

Lobby manager owns lifetime and channels; it does not know about Axum or sockets.

## Game loop: `game.rs`

```rust
pub async fn run_game_loop(
    mut rx_input: mpsc::Receiver<PlayerInput>,
    tx_out: mpsc::Sender<ServerMessage>,
    config: Config,
) {
    let mut state = GameState::new(config.map_size);
    let tick = Duration::from_secs_f32(1.0 / config.tick_rate);

    loop {
        let frame_start = Instant::now();
        while let Ok(input) = rx_input.try_recv() {
            apply_input(&mut state, input);
        }

        systems::movement::update(&mut state, tick);
        systems::combat::update(&mut state, tick);

        let snapshot = protocol::ServerMessage::from_state(&state, frame_start.elapsed());
        if tx_out.send(snapshot).await.is_err() {
            break;
        }

        sleep_until(frame_start + tick).await;
    }
}
```

`game.rs` depends only on domain and time primitives; no Axum/web concerns.

## Protocol translation: `protocol.rs`

```rust
#[derive(Serialize, Deserialize)]
pub enum ClientMessage { Join { lobby_id: String, username: String }, Input(PlayerInput), Ping }

#[derive(Serialize, Deserialize)]
pub enum ServerMessage { WorldSnapshot { players: Vec<PlayerData>, projectiles: Vec<ProjectileData>, server_time: f64 }, GameEvent(GameEvent), Pong }

#[derive(Serialize, Deserialize)]
pub enum GameEvent { PlayerJoined { id: u64, name: String }, PlayerLeft { id: u64 }, PlayerDied { victim_id: u64, killer_id: u64 } }
```

Only the protocol layer should know about serde and wire schemas; adapters convert between
these DTOs and domain types.

## Refactor checklist

- `main.rs` contains no game rules or serde.
- `net.rs` owns Axum routes and WebSocket plumbing only.
- `lobby.rs` owns per-lobby lifecycle and channels.
- `game.rs` runs the loop with domain types and systems only.
- `protocol.rs` defines on-the-wire message contracts.
- `systems/*` remain framework-free and operate solely on `GameState`.
