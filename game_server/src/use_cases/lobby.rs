// Lobby orchestration for spawning and managing game worlds.

use crate::use_cases::game::world_task;
use crate::use_cases::{GameEvent, ServerState, WorldUpdate};
use axum::extract::ws::Utf8Bytes;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, watch, RwLock};

/// Shared configuration for spawning lobby worlds.
#[derive(Debug, Clone)]
pub struct LobbySettings {
    /// Capacity for inbound player input events.
    pub input_channel_capacity: usize,
    /// Capacity for broadcast world updates.
    pub world_broadcast_capacity: usize,
    /// Fixed tick interval for the game loop.
    pub tick_interval: Duration,
}

/// Errors returned by lobby registry operations.
#[derive(Debug)]
pub enum LobbyError {
    /// Lobby already exists and cannot be re-created.
    AlreadyExists,
}

/// Per-lobby channels and access rules.
#[derive(Clone)]
pub struct LobbyHandle {
    /// Identifier clients use to target this lobby.
    pub lobby_id: Arc<str>,
    /// Sender for game events into the lobby world task.
    pub input_tx: mpsc::Sender<GameEvent>,
    /// Broadcast sender for raw world updates.
    pub world_tx: broadcast::Sender<WorldUpdate>,
    /// Broadcast sender for serialized world updates.
    pub world_bytes_tx: broadcast::Sender<Utf8Bytes>,
    /// Watch sender holding the latest serialized world update.
    pub world_latest_tx: watch::Sender<Utf8Bytes>,
    /// Watch sender for high-level server state changes.
    pub server_state_tx: watch::Sender<ServerState>,
    /// Players allowed to spawn into the lobby (empty means open lobby).
    allowed_players: Arc<HashSet<u64>>,
}

impl LobbyHandle {
    /// Returns true if the provided player id should spawn in the lobby.
    pub fn is_player_allowed(&self, player_id: u64) -> bool {
        self.allowed_players.is_empty() || self.allowed_players.contains(&player_id)
    }
}

/// Thread-safe registry for active lobbies.
#[derive(Debug)]
pub struct LobbyRegistry {
    /// Global settings applied to newly created lobbies.
    settings: LobbySettings,
    /// Map of lobby id to active handle.
    lobbies: RwLock<HashMap<String, LobbyHandle>>,
}

impl LobbyRegistry {
    /// Creates a new registry with the provided settings.
    pub fn new(settings: LobbySettings) -> Self {
        Self {
            settings,
            lobbies: RwLock::new(HashMap::new()),
        }
    }

    /// Creates a new lobby and spawns its world task.
    pub async fn create_lobby(
        &self,
        lobby_id: String,
        allowed_players: HashSet<u64>,
    ) -> Result<LobbyHandle, LobbyError> {
        let mut lobbies = self.lobbies.write().await;
        if lobbies.contains_key(&lobby_id) {
            return Err(LobbyError::AlreadyExists);
        }

        // Channel wiring for the lobby world loop.
        let (input_tx, input_rx) = mpsc::channel::<GameEvent>(self.settings.input_channel_capacity);
        let (world_tx, _world_rx) =
            broadcast::channel::<WorldUpdate>(self.settings.world_broadcast_capacity);
        let (world_bytes_tx, _world_bytes_rx) =
            broadcast::channel::<Utf8Bytes>(self.settings.world_broadcast_capacity);
        let (world_latest_tx, _world_latest_rx) = watch::channel::<Utf8Bytes>(Utf8Bytes::from(""));
        let (server_state_tx, _server_state_rx) =
            watch::channel::<ServerState>(ServerState::Lobby);

        // Spawn the authoritative world loop for this lobby.
        tokio::spawn(world_task(
            input_rx,
            world_tx.clone(),
            server_state_tx.clone(),
            self.settings.tick_interval,
        ));

        let lobby = LobbyHandle {
            lobby_id: Arc::from(lobby_id.clone()),
            input_tx,
            world_tx,
            world_bytes_tx,
            world_latest_tx,
            server_state_tx,
            allowed_players: Arc::new(allowed_players),
        };

        lobbies.insert(lobby_id, lobby.clone());
        Ok(lobby)
    }

    /// Returns a lobby handle for the provided id, if it exists.
    pub async fn get_lobby(&self, lobby_id: &str) -> Option<LobbyHandle> {
        let lobbies = self.lobbies.read().await;
        lobbies.get(lobby_id).cloned()
    }
}
