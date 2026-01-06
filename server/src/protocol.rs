use crate::state::{EntityState, ProjectileState, ServerState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum GameEvent {
    Join { player_id: u64 },
    Leave { player_id: u64 },
    Input { player_id: u64, input: PlayerInput },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Identity { player_id: u64 },
    WorldUpdate(WorldUpdate),
    GameState(ServerState),
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerInput {
    #[serde(default)]
    pub thrust: f32,
    #[serde(default)]
    pub turn: f32,
    #[serde(default)]
    pub shoot: bool,
    // #[serde(default)]
    // pub special: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldUpdate {
    pub tick: u64,
    pub entities: Vec<EntityState>,
    #[serde(default)]
    pub projectiles: Vec<ProjectileState>,
}
