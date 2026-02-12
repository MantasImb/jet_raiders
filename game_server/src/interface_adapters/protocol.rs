// Wire protocol DTOs and conversions for public game server messages.
// Internal service-to-service DTOs should live outside this module.

use crate::domain::{EntitySnapshot, PlayerInput, ProjectileSnapshot};
use crate::use_cases::{ServerState, WorldUpdate};
use serde::{Deserialize, Serialize};

/// Messages the server sends to connected clients over the WebSocket.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ServerMessage {
    // Assigned identity for the connection after Join is accepted.
    Identity { player_id: String },
    // Snapshot of the world for a given tick.
    WorldUpdate(WorldUpdateDto),
    // High-level server state transitions (lobby, match start/end).
    GameState(ServerStateDto),
}

/// Messages the client sends to the server over the WebSocket.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum ClientMessage {
    // Initial handshake message with identity metadata.
    Join(JoinPayload),
    // Input messages sent after a successful Join.
    Input(PlayerInputDto),
}

/// Payload for the Join handshake with identity metadata.
#[derive(Debug, Clone, Deserialize)]
pub struct JoinPayload {
    pub guest_id: String,
    pub display_name: String,
}

/// Per-tick input payload sent by the client after joining.
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

/// Snapshot of the world sent to clients on each tick.
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
            entities: update.entities.iter().map(EntityStateDto::from).collect(),
            projectiles: update
                .projectiles
                .iter()
                .map(ProjectileStateDto::from)
                .collect(),
        }
    }
}

/// Flattened entity state for wire transmission in world updates.
#[derive(Debug, Clone, Serialize)]
pub struct EntityStateDto {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
    pub hp: i32,
}

impl From<&EntitySnapshot> for EntityStateDto {
    fn from(entity: &EntitySnapshot) -> Self {
        Self {
            id: entity.id.clone(),
            x: entity.x,
            y: entity.y,
            rot: entity.rot,
            hp: entity.hp,
        }
    }
}

/// Flattened projectile state for wire transmission in world updates.
#[derive(Debug, Clone, Serialize)]
pub struct ProjectileStateDto {
    pub id: String,
    pub owner_id: String,
    pub x: f32,
    pub y: f32,
    pub rot: f32,
}

impl From<&ProjectileSnapshot> for ProjectileStateDto {
    fn from(projectile: &ProjectileSnapshot) -> Self {
        Self {
            id: projectile.id.clone(),
            owner_id: projectile.owner_id.clone(),
            x: projectile.x,
            y: projectile.y,
            rot: projectile.rot,
        }
    }
}

/// Server lifecycle state sent to clients for UI flow.
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
