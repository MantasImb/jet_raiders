// Wire protocol DTOs and conversions for game server messages.

use crate::domain::{EntitySnapshot, PlayerInput, ProjectileSnapshot};
use crate::use_cases::{ServerState, WorldUpdate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    Identity { player_id: u64 },
    WorldUpdate(WorldUpdateDto),
    GameState(ServerStateDto),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    // Initial handshake message with identity metadata.
    Join(JoinPayload),
    // Input messages sent after a successful Join.
    Input(PlayerInputDto),
}

#[derive(Debug, Clone, Deserialize)]
pub struct JoinPayload {
    pub guest_id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerInputDto {
    #[serde(default)]
    pub thrust: f32,
    #[serde(default)]
    pub turn: f32,
    #[serde(default)]
    pub shoot: bool,
}

impl From<PlayerInputDto> for PlayerInput {
    fn from(input: PlayerInputDto) -> Self {
        Self {
            thrust: input.thrust,
            turn: input.turn,
            shoot: input.shoot,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WorldUpdateDto {
    pub tick: u64,
    pub entities: Vec<EntityStateDto>,
    #[serde(default)]
    pub projectiles: Vec<ProjectileStateDto>,
}

impl From<WorldUpdate> for WorldUpdateDto {
    fn from(update: WorldUpdate) -> Self {
        Self {
            tick: update.tick,
            entities: update
                .entities
                .iter()
                .map(EntityStateDto::from)
                .collect(),
            projectiles: update
                .projectiles
                .iter()
                .map(ProjectileStateDto::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EntityStateDto {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    pub hp: i32,
}

impl From<&EntitySnapshot> for EntityStateDto {
    fn from(entity: &EntitySnapshot) -> Self {
        Self {
            id: entity.id,
            x: entity.x,
            y: entity.y,
            rot: entity.rot,
            hp: entity.hp,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectileStateDto {
    pub id: u64,
    pub owner_id: u64,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
}

impl From<&ProjectileSnapshot> for ProjectileStateDto {
    fn from(projectile: &ProjectileSnapshot) -> Self {
        Self {
            id: projectile.id,
            owner_id: projectile.owner_id,
            x: projectile.x,
            y: projectile.y,
            rot: projectile.rot,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ServerStateDto {
    Lobby,
    MatchStarting { in_seconds: u32 },
    MatchRunning,
    MatchEnded,
}

impl From<ServerState> for ServerStateDto {
    fn from(state: ServerState) -> Self {
        match state {
            ServerState::Lobby => ServerStateDto::Lobby,
            ServerState::MatchStarting { in_seconds } => {
                ServerStateDto::MatchStarting { in_seconds }
            }
            ServerState::MatchRunning => ServerStateDto::MatchRunning,
            ServerState::MatchEnded => ServerStateDto::MatchEnded,
        }
    }
}
