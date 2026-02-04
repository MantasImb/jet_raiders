// Lobby orchestration for spawning and managing game worlds.

use crate::use_cases::game::world_task;
use crate::use_cases::{GameEvent, ServerState, WorldUpdate};
use axum::extract::ws::Utf8Bytes;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tokio::sync::{Notify, RwLock, broadcast, mpsc, watch};
use tracing::{debug, info, warn};

/// Shared configuration for spawning lobby worlds.
#[derive(Debug, Clone)]
pub struct LobbySettings {
    /// Capacity for inbound player input events.
    pub input_channel_capacity: usize,
    /// Capacity for broadcast world updates.
    pub world_broadcast_capacity: usize,
    /// Fixed tick interval for the game loop.
    pub tick_interval: Duration,
    /// Default match duration for non-pinned lobbies.
    pub default_match_time_limit: Duration,
}

/// Errors returned by lobby registry operations.
#[derive(Debug)]
pub enum LobbyError {
    /// Lobby already exists and cannot be re-created.
    AlreadyExists,
}

/// Per-lobby channels and access rules.
#[derive(Clone, Debug)]
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
    /// Active connections for this lobby (players + spectators).
    pub active_connections: Arc<AtomicUsize>,
    /// True if the lobby should never be deleted.
    pub is_pinned: bool,
    /// Shutdown signal for the world task.
    pub shutdown_tx: Arc<Notify>,
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
    lobbies: RwLock<HashMap<String, LobbyEntry>>,
}

#[derive(Debug)]
struct LobbyEntry {
    // The externally shared handle for this lobby.
    handle: LobbyHandle,
    // Track the world task for debugging/visibility.
    #[allow(dead_code)]
    world_task: tokio::task::JoinHandle<()>,
}

impl LobbyRegistry {
    /// Creates a new registry with the provided settings.
    pub fn new(settings: LobbySettings) -> Self {
        Self {
            settings,
            lobbies: RwLock::new(HashMap::new()),
        }
    }

    /// Returns the default match time limit for non-pinned lobbies.
    pub fn default_match_time_limit(&self) -> Duration {
        self.settings.default_match_time_limit
    }

    /// Creates a new lobby and spawns its world task.
    pub async fn create_lobby(
        &self,
        lobby_id: String,
        allowed_players: HashSet<u64>,
        is_pinned: bool,
        match_time_limit: Duration,
    ) -> Result<LobbyHandle, LobbyError> {
        let mut lobbies = self.lobbies.write().await;
        if lobbies.contains_key(&lobby_id) {
            // Trace duplicate lobby creation attempts for visibility.
            warn!(lobby_id = %lobby_id, "lobby already exists");
            return Err(LobbyError::AlreadyExists);
        }

        // Channel wiring for the lobby world loop.
        let (input_tx, input_rx) = mpsc::channel::<GameEvent>(self.settings.input_channel_capacity);
        let (world_tx, _world_rx) =
            broadcast::channel::<WorldUpdate>(self.settings.world_broadcast_capacity);
        let (world_bytes_tx, _world_bytes_rx) =
            broadcast::channel::<Utf8Bytes>(self.settings.world_broadcast_capacity);
        let (world_latest_tx, _world_latest_rx) = watch::channel::<Utf8Bytes>(Utf8Bytes::from(""));
        let (server_state_tx, _server_state_rx) = watch::channel::<ServerState>(ServerState::Lobby);

        // Shutdown signal for the world task.
        let shutdown_tx = Arc::new(Notify::new());

        // Spawn the authoritative world loop for this lobby.
        let world_task = tokio::spawn(world_task(
            input_rx,
            world_tx.clone(),
            server_state_tx.clone(),
            self.settings.tick_interval,
            shutdown_tx.clone(),
            match_time_limit,
        ));

        let lobby = LobbyHandle {
            lobby_id: Arc::from(lobby_id.clone()),
            input_tx,
            world_tx,
            world_bytes_tx,
            world_latest_tx,
            server_state_tx,
            active_connections: Arc::new(AtomicUsize::new(0)),
            is_pinned,
            shutdown_tx,
            allowed_players: Arc::new(allowed_players),
        };

        lobbies.insert(
            lobby_id,
            LobbyEntry {
                handle: lobby.clone(),
                world_task,
            },
        );
        // Log lobby creation for lifecycle visibility.
        info!(
            lobby_id = %lobby.lobby_id,
            is_pinned,
            match_time_limit_secs = match_time_limit.as_secs(),
            "lobby created"
        );
        Ok(lobby)
    }

    /// Spawns a watcher that removes empty lobbies once the match ends.
    pub fn spawn_match_end_watcher(
        self: Arc<Self>,
        lobby_id: Arc<str>,
        mut server_state_rx: watch::Receiver<ServerState>,
    ) {
        tokio::spawn(async move {
            loop {
                if server_state_rx.changed().await.is_err() {
                    // Channel closed; stop watching for match end.
                    debug!(lobby_id = %lobby_id, "server state channel closed");
                    break;
                }

                let state = server_state_rx.borrow().clone();
                if matches!(state, ServerState::MatchEnded) {
                    // If the match ends while empty, clean up immediately.
                    info!(lobby_id = %lobby_id, "match ended; checking for cleanup");
                    self.cleanup_if_empty_on_match_end(&lobby_id).await;
                    break;
                }
            }
        });
    }

    /// Returns a lobby handle for the provided id, if it exists.
    pub async fn get_lobby(&self, lobby_id: &str) -> Option<LobbyHandle> {
        let lobbies = self.lobbies.read().await;
        lobbies.get(lobby_id).map(|entry| entry.handle.clone())
    }

    /// Record a new connection for the lobby.
    pub async fn register_connection(&self, lobby_id: &str) -> Option<LobbyHandle> {
        let lobbies = self.lobbies.read().await;
        let entry = lobbies.get(lobby_id)?;
        // Count all sockets (players + spectators) as active connections.
        entry
            .handle
            .active_connections
            .fetch_add(1, Ordering::SeqCst);
        Some(entry.handle.clone())
    }

    /// Record a disconnect and delete the lobby if it is now empty.
    pub async fn register_disconnect(&self, lobby_id: &str) {
        let mut lobbies = self.lobbies.write().await;
        let Some(entry) = lobbies.get(lobby_id) else {
            // Lobby may have already been removed by another task.
            debug!(lobby_id = %lobby_id, "disconnect for missing lobby");
            return;
        };

        // Decrement the active connection count and check for cleanup.
        let remaining = {
            // Avoid underflow if disconnects race after cleanup.
            let counter = &entry.handle.active_connections;
            let mut current = counter.load(Ordering::SeqCst);
            loop {
                if current == 0 {
                    break 0;
                }
                match counter.compare_exchange(
                    current,
                    current - 1,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                ) {
                    Ok(_) => break current - 1,
                    Err(updated) => current = updated,
                }
            }
        };

        // Spectators keep the lobby alive by design.
        if remaining == 0
            && !entry.handle.is_pinned
            && matches!(
                entry.handle.server_state_tx.borrow().clone(),
                ServerState::MatchEnded
            )
        {
            // Signal the world task to exit, then remove the lobby entry.
            info!(lobby_id = %lobby_id, "lobby empty after match end; shutting down");
            entry.handle.shutdown_tx.notify_waiters();
            lobbies.remove(lobby_id);
        }
    }

    async fn cleanup_if_empty_on_match_end(&self, lobby_id: &str) {
        let mut lobbies = self.lobbies.write().await;
        let Some(entry) = lobbies.get(lobby_id) else {
            return;
        };

        if entry.handle.is_pinned {
            // Pinned lobbies are never removed by cleanup.
            debug!(lobby_id = %lobby_id, "cleanup skipped for pinned lobby");
            return;
        }

        if entry.handle.active_connections.load(Ordering::SeqCst) == 0 {
            // Remove empty lobbies once a match has ended.
            info!(lobby_id = %lobby_id, "lobby empty on match end; shutting down");
            entry.handle.shutdown_tx.notify_waiters();
            lobbies.remove(lobby_id);
        }
    }
}
