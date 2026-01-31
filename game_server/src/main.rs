mod app_state;
mod config;
mod game;
mod lobby;
mod net;
mod protocol;
mod state;
mod systems;
mod tuning;
mod utils;

use crate::app_state::AppState;
use crate::game::world_task;
use crate::net::ws_handler;
use crate::protocol::{GameEvent, WorldUpdate};
use crate::state::ServerState;

use axum::{Router, extract::ws::Utf8Bytes, routing::get};
use std::{net::SocketAddr, sync::Arc};
use tokio::sync::{broadcast, mpsc, watch};

fn init_tracing() {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let json = matches!(std::env::var("LOG_FORMAT").as_deref(), Ok("json"));
    if json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .json()
            .with_current_span(true)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .compact()
            .init();
    }

    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::capture();
        tracing::error!(%info, ?backtrace, "panic");
    }));
}

#[tokio::main]
async fn main() {
    // Load .env locally; safe to ignore when not present.
    let _ = dotenvy::dotenv();
    init_tracing();
    // Setup Channels
    // input_tx/rx: All client inputs go to the single World Task.
    let (input_tx, input_rx) = mpsc::channel::<GameEvent>(config::INPUT_CHANNEL_CAPACITY);

    // world_tx/rx: World updates are broadcast to all clients.
    let (world_tx, _world_rx) = broadcast::channel::<WorldUpdate>(config::WORLD_BROADCAST_CAPACITY);

    // world_bytes_tx/rx: Serialized world updates shared across all clients.
    let (world_bytes_tx, _world_bytes_rx) =
        broadcast::channel::<Utf8Bytes>(config::WORLD_BROADCAST_CAPACITY);
    let (world_latest_tx, _world_latest_rx) = watch::channel::<Utf8Bytes>(Utf8Bytes::from(""));

    // server_state_tx: High-level state (Lobby, MatchRunning) changes.
    let (server_state_tx, _server_state_rx) = watch::channel::<ServerState>(ServerState::Lobby);

    let state = Arc::new(AppState {
        input_tx,
        world_tx,
        world_bytes_tx,
        world_latest_tx,
        server_state_tx,
    });

    // Spawn the Game Loop (World Task)
    // This runs independently in its own thread/task.
    tokio::spawn(world_task(
        input_rx,
        state.world_tx.clone(),
        state.server_state_tx.clone(),
    ));

    // Spawn the world update serializer task in the adapter layer.
    tokio::spawn(net::world_update_serializer(
        state.world_tx.subscribe(),
        state.world_bytes_tx.clone(),
        state.world_latest_tx.clone(),
    ));

    // Start the Web Server
    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    tracing::info!(%addr, "listening");

    // Bind TCP listener with error handling
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!(%addr, error = %e, "failed to bind");
            return; // Abort startup on bind failure
        }
    };

    // Serve app and report errors rather than panicking
    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "server error");
    }
}
