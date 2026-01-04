use crate::protocol::{GameEvent, WorldUpdate};
use serde::Serialize;
use tokio::sync::{broadcast, mpsc, watch};

#[derive(Clone)]
pub struct AppState {
    pub input_tx: mpsc::Sender<GameEvent>,
    pub world_tx: broadcast::Sender<WorldUpdate>,
    pub server_state_tx: watch::Sender<ServerState>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityState {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
}

#[derive(Debug, Clone, Serialize)]
pub enum ServerState {
    Lobby,
    MatchStarting { in_seconds: u32 },
    MatchRunning,
    MatchEnded,
}
