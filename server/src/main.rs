mod config;
mod game;
mod lobby;
mod net;
mod protocol;
mod state;
mod utils;

use crate::game::world_task;
use crate::net::ws_handler;
use crate::protocol::{GameEvent, WorldUpdate};
use crate::state::{AppState, ServerState};

use axum::{Router, routing::get};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{broadcast, mpsc, watch};

#[tokio::main]
async fn main() {
    // Setup Channels
    // input_tx/rx: All client inputs go to the single World Task.
    let (input_tx, input_rx) = mpsc::channel::<GameEvent>(1024);

    // world_tx/rx: World updates are broadcast to all clients.
    let (world_tx, _world_rx) = broadcast::channel::<WorldUpdate>(128);

    // server_state_tx: High-level state (Lobby, MatchRunning) changes.
    let (server_state_tx, _server_state_rx) = watch::channel::<ServerState>(ServerState::Lobby);

    let state = Arc::new(AppState {
        input_tx,
        world_tx,
        server_state_tx,
    });

    // Spawn the Game Loop (World Task)
    // This runs independently in its own thread/task.
    tokio::spawn(world_task(
        input_rx,
        state.world_tx.clone(),
        state.server_state_tx.clone(),
    ));

    // Start the Web Server
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {addr}");

    // Bind TCP listener with error handling
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("failed to bind {}: {}", addr, e);
            return; // Abort startup on bind failure
        }
    };

    // Serve app and report errors rather than panicking
    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("server error: {}", e);
    }
}
