use crate::use_cases::{GameEvent, ServerState, WorldUpdate};
use axum::extract::ws::Utf8Bytes;
use tokio::sync::{broadcast, mpsc, watch};

#[derive(Clone)]
pub struct AppState {
    // Inputs flowing from the network into the game loop.
    pub input_tx: mpsc::Sender<GameEvent>,
    // World updates produced by the game loop (domain structs).
    pub world_tx: broadcast::Sender<WorldUpdate>,
    // Serialized world updates, shared across all connections.
    pub world_bytes_tx: broadcast::Sender<Utf8Bytes>,
    // Latest serialized world update for lag recovery.
    pub world_latest_tx: watch::Sender<Utf8Bytes>,
    // High-level server state (lobby/match).
    pub server_state_tx: watch::Sender<ServerState>,
}
