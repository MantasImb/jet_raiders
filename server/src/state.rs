use crate::protocol::{GameEvent, PlayerInput, WorldUpdate};
use serde::Serialize;
use tokio::sync::{broadcast, mpsc, watch};

#[derive(Clone)]
pub struct AppState {
    pub input_tx: mpsc::Sender<GameEvent>,
    pub world_tx: broadcast::Sender<WorldUpdate>,
    pub server_state_tx: watch::Sender<ServerState>,
}

#[derive(Debug, Clone, Serialize)]
pub enum ServerState {
    Lobby,
    MatchStarting { in_seconds: u32 },
    MatchRunning,
    MatchEnded,
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityState {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectileState {
    pub id: u64,
    pub owner_id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
}

pub struct SimEntity {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,

    // Movement-only state (do not serialize to clients)
    pub throttle: f32,           // 0.0..=1.0
    pub last_input: PlayerInput, // last received input for this entity
    pub shoot_cooldown: f32,     // seconds until next allowed shot
}

pub struct SimProjectile {
    pub id: u64,
    pub owner_id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    pub vx: f32,
    pub vy: f32,
    pub ttl: f32,
}

impl From<&SimEntity> for EntityState {
    fn from(e: &SimEntity) -> Self {
        Self {
            id: e.id,
            x: e.x,
            y: e.y,
            rot: e.rot,
        }
    }
}

impl From<&SimProjectile> for ProjectileState {
    fn from(p: &SimProjectile) -> Self {
        Self {
            id: p.id,
            owner_id: p.owner_id,
            x: p.x,
            y: p.y,
            rot: p.rot,
        }
    }
}
