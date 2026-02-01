// Use-case level inputs/outputs for the game loop.

use crate::domain::{EntitySnapshot, PlayerInput, ProjectileSnapshot};

#[derive(Debug, Clone)]
pub enum GameEvent {
    Join { player_id: u64 },
    Leave { player_id: u64 },
    Input { player_id: u64, input: PlayerInput },
}

#[derive(Debug, Clone)]
pub enum ServerState {
    Lobby,
    MatchStarting { in_seconds: u32 },
    MatchRunning,
    MatchEnded,
}

#[derive(Debug, Clone)]
pub struct WorldUpdate {
    pub tick: u64,
    pub entities: Vec<EntitySnapshot>,
    pub projectiles: Vec<ProjectileSnapshot>,
}
