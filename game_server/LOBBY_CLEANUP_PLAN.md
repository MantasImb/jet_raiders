# Lobby Cleanup Plan

## Goal

Add lobby/world task cleanup so that lobbies are removed immediately after the
last connection disconnects. Spectators still count as active connections and
keep the lobby alive. The default `test` lobby is pinned and must never be
deleted.

## Policy

- A lobby is deleted when `active_connections` reaches zero **and** the match
  has ended.
- Spectators count as active connections (explicitly keep this behavior).
- The `test` lobby is pinned and is never deleted.
- Cleanup is immediate after `MatchEnded` (no idle timeout).
- Match end is currently driven by a time limit; `0` means infinite.

## Design Changes

### Lobby Registry

- Track `active_connections` per lobby.
- Track `is_pinned` (true for `test`).
- Add a `shutdown_tx` (signal) for the world task to exit.
- Add metadata needed for cleanup and removal.
- Spawn a watcher that listens for `MatchEnded` and triggers cleanup if the
  lobby is empty at that time.

**Proposed metadata**

```rust
pub struct LobbyHandle {
    pub lobby_id: Arc<str>,
    pub input_tx: mpsc::Sender<GameEvent>,
    pub world_tx: broadcast::Sender<WorldUpdate>,
    pub world_bytes_tx: broadcast::Sender<Utf8Bytes>,
    pub world_latest_tx: watch::Sender<Utf8Bytes>,
    pub server_state_tx: watch::Sender<ServerState>,
    pub active_connections: Arc<AtomicUsize>,
    pub is_pinned: bool,
    pub shutdown_tx: Arc<Notify>,
}

struct LobbyEntry {
    handle: LobbyHandle,
    world_task: tokio::task::JoinHandle<()>,
}
```

**Rationale**

- `active_connections` counts all sockets (players + spectators) to match the
  policy of keeping the lobby alive while any client is connected.
- `is_pinned` protects the `test` lobby from deletion.
- `shutdown_tx` (e.g., `Notify`) allows the world task to exit immediately when
  the lobby is removed.
- `world_task` is optional but useful for logging and debugging shutdown.

### World Task

- Accept a shutdown signal and exit the loop when triggered.
- Allow channels to close naturally so serializer tasks can exit.
- Track match time elapsed and emit `ServerState::MatchEnded` once the limit
  is reached (unless the limit is `0`).

```rust
pub async fn world_task(
    mut input_rx: mpsc::Receiver<GameEvent>,
    world_tx: broadcast::Sender<WorldUpdate>,
    server_state_tx: watch::Sender<ServerState>,
    tick_interval: Duration,
    shutdown: Arc<Notify>,
    match_time_limit: Duration,
) {
    let mut interval = tokio::time::interval(tick_interval);
    let mut match_elapsed = Duration::from_secs(0);
    let mut match_ended = false;

    loop {
        tokio::select! {
            _ = shutdown.notified() => break,
            _ = interval.tick() => {
                if !match_ended && match_time_limit != Duration::from_secs(0) {
                    match_elapsed += tick_interval;
                    if match_elapsed >= match_time_limit {
                        let _ = server_state_tx.send(ServerState::MatchEnded);
                        match_ended = true;
                    }
                }
                // existing tick logic
            }
        }
    }
}
```

### Networking

- Increment `active_connections` after successful connection bootstrap.
- Decrement on disconnect cleanup.
- When `active_connections` reaches zero and the match has ended (and the lobby
  is not pinned), remove the lobby from the registry and signal shutdown.
- If the match ends and the lobby is already empty, the match-end watcher
  removes the lobby immediately.

```rust
// On connect:
lobby.active_connections.fetch_add(1, Ordering::SeqCst);

// On disconnect:
let remaining = lobby
    .active_connections
    .fetch_sub(1, Ordering::SeqCst)
    .saturating_sub(1);
if remaining == 0 && match_has_ended && !lobby.is_pinned {
    lobby.shutdown_tx.notify_waiters();
    registry.remove(lobby_id);
}
```

## Files To Update

- `game_server/src/use_cases/lobby.rs`
- `game_server/src/use_cases/game.rs`
- `game_server/src/interface_adapters/net.rs`
- `game_server/src/frameworks/server.rs`

## Logging

- Log lobby creation and deletion with lobby id and connection counts.
- Log shutdown signal handling in the world task.

## Verification

- Connect and disconnect a client; confirm lobby removed on last disconnect.
- Confirm spectators keep lobby alive.
- Confirm `test` lobby remains after all clients disconnect.
