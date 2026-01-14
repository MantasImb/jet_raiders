# Hybrid Matchmaking Adapter (Option 3)

This document expands Option 3 (hybrid matchmaking adapter) and shows how to
integrate it into the current server layout. The goal is to keep world tasks
per lobby while making the lobby assignment decision pluggable.

## Goals

- Keep `LobbyManager` authoritative for per-lobby world tasks.
- Allow matchmaking to be local now and remote later without refactoring
  the game loop or networking code.
- Preserve clean architecture boundaries (`lobby.rs` for orchestration,
  `net.rs` for adapters, `game.rs` for the world task).

## High-Level Design

Introduce a `MatchmakingProvider` trait (use-case layer) that returns a lobby
assignment. The provider can be:

- `LocalMatchmaker` (in-process random ID, current default).
- `RemoteMatchmaker` (calls an external service, future-ready).

`net.rs` asks for an assignment after the join message is received. Then it
routes the socket to the lobby returned by the provider.

## Proposed Message Flow

1. Client connects to `/ws`.
2. Client sends `JoinRandom` (or `Join { lobby_id }`).
3. `net.rs` calls `MatchmakingProvider`.
4. Provider returns `LobbyAssignment { lobby_id }`.
5. `LobbyManager` creates or joins that lobby.
6. `net.rs` wires the socket to that lobby's input/output channels.

## Data Types (Protocol and Domain)

Add these to `server/src/protocol.rs`:

```rust
// Wire message for requesting random assignment.
// Keep this in `protocol.rs` as it is client-facing.
pub enum ClientMessage {
    // Existing variants...
    Join { lobby_id: String, username: String },
    JoinRandom { username: String }, // Client requests assignment.
}
```

Add these to `server/src/lobby.rs` or a new `matchmaking.rs` module:

```rust
// Domain-level response used by the lobby manager and net adapter.
pub struct LobbyAssignment {
    pub lobby_id: String,
}

// Use-case boundary for pluggable matchmaking providers.
pub trait MatchmakingProvider: Send + Sync {
    fn assign_lobby(&self, username: &str) -> LobbyAssignment;
}
```

## Lobby Manager Wiring

Add a `LobbyManager` that owns the registry and world task spawning. It exposes
a method to get or create the lobby by ID.

```rust
// Keeps per-lobby channels and spawns world tasks on demand.
pub struct LobbyManager {
    // Use a map keyed by lobby ID for quick lookup.
    lobbies: std::collections::HashMap<String, LobbyHandle>,
}

// Handle returned to `net.rs` for routing a connection.
pub struct LobbyHandle {
    pub input_tx: tokio::sync::mpsc::Sender<crate::protocol::GameEvent>,
    pub world_bytes_tx: tokio::sync::broadcast::Sender<axum::extract::ws::Utf8Bytes>,
}

impl LobbyManager {
    pub fn get_or_create(&mut self, lobby_id: &str) -> LobbyHandle {
        // Check for existing lobby first to keep IDs stable.
        if let Some(handle) = self.lobbies.get(lobby_id) {
            return handle.clone();
        }

        // Create per-lobby channels and spawn the world task.
        let (input_tx, input_rx) =
            tokio::sync::mpsc::channel(crate::config::INPUT_CHANNEL_CAPACITY);
        let (world_tx, _world_rx) =
            tokio::sync::broadcast::channel(crate::config::WORLD_BROADCAST_CAPACITY);

        // Spawn the world task for this lobby.
        tokio::spawn(crate::game::world_task(
            input_rx,
            world_tx.clone(),
            /* server_state_tx */ todo!(),
        ));

        // Serialize world updates in the adapter layer.
        let (world_bytes_tx, _world_bytes_rx) =
            tokio::sync::broadcast::channel(crate::config::WORLD_BROADCAST_CAPACITY);
        tokio::spawn(crate::net::world_update_serializer(
            world_tx.subscribe(),
            world_bytes_tx.clone(),
        ));

        // Store and return the new handle.
        let handle = LobbyHandle { input_tx, world_bytes_tx };
        self.lobbies.insert(lobby_id.to_string(), handle.clone());
        handle
    }
}
```

Notes:

- `LobbyHandle` is cloned per connection to avoid sharing a single receiver.
- You will want to store `server_state_tx` per lobby similarly to the current
  single-world implementation.
- Use a cleanup policy (idle timer or empty-lobby eviction) to avoid leaks.

## Local Provider (Default)

This provider stays in-process and uses a random lobby ID. It keeps the ID
format stable so that a future remote service can adopt the same format.

```rust
// Generates random lobby IDs using existing RNG utilities.
pub struct LocalMatchmaker {
    // Pre-allocated alphabet for base32 or base64 ID creation.
    alphabet: &'static [u8],
}

impl LocalMatchmaker {
    pub fn new() -> Self {
        Self { alphabet: b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567" }
    }

    pub fn generate_lobby_id(&self, len: usize) -> String {
        // Generate a short, user-friendly ID.
        let mut id = String::with_capacity(len);
        for _ in 0..len {
            // Use an existing RNG helper; replace with a stronger RNG if needed.
            let idx = (crate::utils::rng::rand_id() % self.alphabet.len() as u64) as usize;
            id.push(self.alphabet[idx] as char);
        }
        id
    }
}

impl MatchmakingProvider for LocalMatchmaker {
    fn assign_lobby(&self, _username: &str) -> LobbyAssignment {
        // Pick a random ID; `LobbyManager` will ensure no collisions.
        LobbyAssignment { lobby_id: self.generate_lobby_id(6) }
    }
}
```

## Net Adapter Integration

Handle `JoinRandom` by calling the matchmaking provider and routing to that
lobby. Keep all socket operations in `net.rs`.

```rust
// Convert a join request into a lobby assignment and lobby handle.
fn resolve_lobby(
    msg: crate::protocol::ClientMessage,
    matchmaker: &dyn MatchmakingProvider,
    lobby_manager: &mut LobbyManager,
) -> LobbyHandle {
    match msg {
        crate::protocol::ClientMessage::Join { lobby_id, .. } => {
            // Use the explicit lobby ID requested by the client.
            lobby_manager.get_or_create(&lobby_id)
        }
        crate::protocol::ClientMessage::JoinRandom { username } => {
            // Ask the provider for a lobby assignment.
            let assignment = matchmaker.assign_lobby(&username);
            lobby_manager.get_or_create(&assignment.lobby_id)
        }
        _ => {
            // Reject invalid handshake messages.
            // You can return an error type here in the real implementation.
            panic!("invalid join message");
        }
    }
}
```

## Configuration and Future Remote Provider

Use a config switch to select the provider:

- `MATCHMAKING_MODE=local`
- `MATCHMAKING_MODE=remote`

In the remote implementation, `assign_lobby` will call a service (HTTP or gRPC)
and return the assigned `lobby_id`. The rest of the server stays unchanged.

## Files to Touch

- `server/src/protocol.rs`: add `JoinRandom`.
- `server/src/lobby.rs`: add `LobbyManager`, `LobbyHandle`, and optional
  matchmaking trait if you keep it there.
- `server/src/net.rs`: call the matchmaker and lobby manager during handshake.
- `server/src/main.rs`: construct the matchmaker and lobby manager and pass
  them into `AppState`.
- `server/src/app_state.rs`: store shared references to the manager and provider.

## Testing Considerations

- Unit test `LocalMatchmaker` ID format and length.
- Unit test `LobbyManager` uniqueness behavior.
- Integration test that `JoinRandom` returns a valid lobby and spawns a world
  task.
