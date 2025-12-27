use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{broadcast, mpsc, watch};

#[derive(Clone)]
struct AppState {
    input_tx: mpsc::Sender<InputEvent>,
    world_tx: broadcast::Sender<WorldUpdate>,
    server_state_tx: watch::Sender<ServerState>,
}

#[derive(Debug, Clone, Serialize)]
struct WorldUpdate {
    tick: u64,
    // For real games you’d send deltas or a filtered snapshot
    entities: Vec<EntityState>,
}

#[derive(Debug, Clone, Serialize)]
struct EntityState {
    id: u32,
    x: f32,
    y: f32,
    rot: f32,
}

#[derive(Debug, Clone, Serialize)]
enum ServerState {
    Lobby,
    MatchStarting { in_seconds: u32 },
    MatchRunning,
    MatchEnded,
}

#[derive(Debug, Clone)]
struct InputEvent {
    player_id: u64,
    input: PlayerInput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "data")]
enum PlayerInput {
    Move { dx: f32, dy: f32 },
    Rotate { drot: f32 },
}

#[tokio::main]
async fn main() {
    // Inputs: many client tasks -> one world task
    let (input_tx, input_rx) = mpsc::channel::<InputEvent>(1024);

    // World updates: one world task -> many clients
    let (world_tx, _world_rx) = broadcast::channel::<WorldUpdate>(128);

    // Server state: one producer -> many clients (latest only)
    let (server_state_tx, _server_state_rx) = watch::channel::<ServerState>(ServerState::Lobby);

    let state = Arc::new(AppState {
        input_tx,
        world_tx,
        server_state_tx,
    });

    tokio::spawn(world_task(
        input_rx,
        state.world_tx.clone(),
        state.server_state_tx.clone(),
    ));

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    // For demo purposes only. Real code: authenticate and assign player IDs properly.
    let player_id = rand_id();

    let mut world_rx = state.world_tx.subscribe();
    let mut server_state_rx = state.server_state_tx.subscribe();

    // Send current server state immediately (watch has a current value).
    let initial = server_state_rx.borrow().clone();
    if send_server_state(&mut socket, &initial).await.is_err() {
        return;
    }

    loop {
        tokio::select! {
            // Client -> server: inputs
            incoming = socket.recv() => {
                let Some(Ok(msg)) = incoming else { break; };

                match msg {
                    Message::Text(text) => {
                        // Expect JSON inputs, e.g.:
                        // {"type":"Move","data":{"dx":1.0,"dy":0.0}}
                        if let Ok(input) = serde_json::from_str::<PlayerInput>(&text) {
                            // Avoid stalling the connection task if the input queue is full.
                            // For a game, dropping inputs under load is often preferable
                            // to backpressuring reads.
                            let _ = state.input_tx.try_send(InputEvent { player_id, input });
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }

            // World -> client: broadcast updates
            world_msg = world_rx.recv() => {
                match world_msg {
                    Ok(update) => {
                        if send_world_update(&mut socket, &update).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_n)) => {
                        // Client is too slow and missed some updates.
                        // You might send a full snapshot here instead.
                        continue;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            // Server state -> client: match lifecycle etc. (latest only)
            changed = server_state_rx.changed() => {
                if changed.is_err() {
                    break;
                }
                let st = server_state_rx.borrow().clone();
                if send_server_state(&mut socket, &st).await.is_err() {
                    break;
                }
            }
        }
    }

    println!("client {player_id} disconnected");
}

async fn send_world_update(
    socket: &mut WebSocket,
    update: &WorldUpdate,
) -> Result<(), axum::Error> {
    let txt = serde_json::to_string(update).unwrap();
    socket.send(Message::Text(txt.into())).await
}

async fn send_server_state(socket: &mut WebSocket, state: &ServerState) -> Result<(), axum::Error> {
    let txt = serde_json::to_string(state).unwrap();
    socket.send(Message::Text(txt.into())).await
}

async fn world_task(
    mut input_rx: mpsc::Receiver<InputEvent>,
    world_tx: broadcast::Sender<WorldUpdate>,
    server_state_tx: watch::Sender<ServerState>,
) {
    // Demo world state
    let mut tick: u64 = 0;
    let mut entities = vec![
        EntityState {
            id: 1,
            x: 0.0,
            y: 0.0,
            rot: 0.0,
        },
        EntityState {
            id: 2,
            x: 5.0,
            y: 2.0,
            rot: 0.0,
        },
    ];

    // Demo “match lifecycle”
    let _ = server_state_tx.send(ServerState::MatchStarting { in_seconds: 3 });
    tokio::time::sleep(Duration::from_secs(3)).await;
    let _ = server_state_tx.send(ServerState::MatchRunning);

    let mut interval = tokio::time::interval(Duration::from_millis(2000)); // 50ms

    loop {
        interval.tick().await;

        // Drain all currently queued inputs without blocking the tick.
        while let Ok(ev) = input_rx.try_recv() {
            // Apply input to demo entity (id=1 for simplicity)
            let e = &mut entities[0];
            match ev.input {
                PlayerInput::Move { dx, dy } => {
                    e.x += dx;
                    e.y += dy;
                }
                PlayerInput::Rotate { drot } => {
                    e.rot += drot;
                }
            }
        }

        tick += 1;

        // Broadcast world update
        let update = WorldUpdate {
            tick,
            entities: entities.clone(),
        };
        let _ = world_tx.send(update);

        // Demo end match
        if tick == 400 {
            let _ = server_state_tx.send(ServerState::MatchEnded);
        }
    }
}

fn rand_id() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}
